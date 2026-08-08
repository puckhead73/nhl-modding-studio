//! **Head / hair / facial-hair census** — Phase 0 of the appearance-catalog rebuild.
//!
//! The old extraction gated a player's head on `pPaD == lOnr` and scanned only the
//! `ubPc` bio table, so it captured ~4 % of heads and *no* hair, facial-hair, staff,
//! or generic meshes ([`crate::assets::scan_player_assets`]). Before rewriting that,
//! we need a **verified inventory** of what actually ships on disk, per title and
//! platform: which folders hold head vs hair vs equipment geometry, how each is
//! named, and — the open question — whether facial hair is ever *distinct geometry*
//! or (as the vanilla heads suggest) painted into the head texture.
//!
//! This module is the re-runnable census engine. It is filename-driven for the full
//! inventory (fast, complete counts) and decodes a capped sample per class for the
//! submesh vocabulary + facial-hair-geometry probe. It writes **no** asset bytes and
//! resolves nothing to player names — it is a structural map, so its output is safe
//! to keep as an audit artifact (the per-file rows carry EA filenames only, so they
//! stay under the QA-out dir, not committed).
//!
//! ## Verified layout (2026-08-07, against the user's Xbox 360 + PS4 mounts)
//!
//! | Class | NHL 07 | NHL 08/09 | NHL 10 → Legacy | PS4 |
//! |---|---|---|---|---|
//! | Player head | `player/player_{id}/head_{id}.rx2` | `player/head_{id}.rx2` | `player/player_{id}.rx2` (hair as `haircap_`/`cards` submesh) | `player/{slug}.mcd`, `player/head_generic_player_{id}.mcd` |
//! | Player hair | with head | **separate** `player/hair_{id}_0_0_0.rx2` | head submesh | **separate** `player/hair_*.mcd` |
//! | Coach head | `coachhead/coachhead_{id}` | `coachhead/head_{id}.rx2` (+ `hair_`, `eyel/eyer/eyes`) | *(no coachhead folder — staff reuse pool heads)* | `characters/coach/` |
//! | Referee/trainer head | `referee_0`/`trainer_0` body only | body only | body only | `characters/{referee,trainer}/` |
//! | Create-a-player | — | `czhead/`,`czhair/` = **textures only** | textures only | in-engine |
//!
//! `czhead`/`czhair` carry only `*_cm`/`*_am` rasters (`eyebrow_*`, `blackeye_*`,
//! `haircp_*`) — create-a-player is texture overlays on a base head, **not** separate
//! head geometry. That is why the classifier tags them `Texture`, never `Head`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project::Platform;

/// Who a mesh belongs to. Derived from the containing folder, refined by the
/// `generic_player` marker (PS4 keeps generic heads *in* the player folder).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Player,
    Generic,
    Coach,
    Referee,
    Trainer,
    Commissioner,
    Timekeeper,
    Unknown,
}

/// What a file *is*. `Head`/`Hair` are the geometry we capture; `Eye` is the
/// eyeball/lash companion; `Body` is a suit/jersey/skate; `Equipment` is a
/// gear mesh sharing the PS4 player folder; `Texture` is any map raster.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Part {
    Head,
    Hair,
    Eye,
    Body,
    Equipment,
    Texture,
    Unknown,
}

/// One classified on-disk file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CensusEntry {
    /// Folder under `rendering/` (RenderWare) or `characters/` (PS4).
    pub folder: String,
    pub file: String,
    pub role: Role,
    pub part: Part,
    /// Whether this is a texture-map raster rather than geometry.
    pub is_texture: bool,
    /// `generic_player_*` / `head_generic_player_*` — a shared pool head/hair.
    pub is_generic: bool,
    /// Numeric id parsed from the name, if any (`player_67` → 67, pool `>= 60000`).
    pub id: Option<u64>,
    /// Submesh names, when the file was deep-decoded (sampled). Empty otherwise.
    pub submeshes: Vec<String>,
    /// Total vertex count across submeshes when decoded; `None` if not decoded.
    pub verts: Option<usize>,
    /// A submesh name matched a facial-hair keyword (beard/moustache/…): distinct
    /// facial-hair geometry actually exists on this asset.
    pub facial_hair_geom: bool,
    /// Set when a sampled decode was attempted but failed (parser refused it).
    pub decode_error: Option<String>,
}

/// Aggregate report for one (title, platform) tree.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CensusReport {
    pub title: String,
    pub platform: String,
    pub entries: Vec<CensusEntry>,
    /// class label → count, e.g. `"player/head"` → 1500.
    pub counts: BTreeMap<String, usize>,
    /// Union of every submesh name seen across sampled decodes → occurrence count.
    pub submesh_vocab: BTreeMap<String, usize>,
    /// Files whose sampled decode found distinct facial-hair geometry.
    pub facial_hair_hits: Vec<String>,
}

/// Map-suffix tokens that mark a `.rx2` as a texture raster, not geometry. Covers
/// the plain single maps (`cm`/`nm`/`sm`/`am`/…) and the `c`-prefixed compound
/// forms seen in the coach/body folders (`cam`/`ccm`/`cnm`/`csm`/`cpm`/`ctm`).
const MAP_SUFFIXES: &[&str] = &[
    "cm", "nm", "sm", "am", "xm", "rm", "dm", "gm", "bm", "pm", "tm", "im", "om", "em", "lm",
    "cam", "ccm", "cnm", "csm", "cpm", "ctm", "cgm", "cdm", "crm",
    // Hair-card raster variants (`hair_{id}_0_aa.rx2`, verified `.Raster` internally):
    // texture atlases, not geometry — the hair *mesh* is the `_0_0_0` file.
    "aa", "ab",
];

/// Equipment name-prefix keywords that share the PS4 `player/` folder (and appear
/// in the RenderWare equipment folders). A file leading with one of these is gear,
/// never a head/hair — this is the head-vs-equipment discriminator the PS4 tree
/// needs (its `player/` folder mixes both).
const EQUIPMENT_KEYWORDS: &[&str] = &[
    "goaliehelmet",
    "goaliepad",
    "goaliestick",
    "playerstick",
    "brokestick",
    "stick",
    "glove",
    "blocker",
    "trapper",
    "helmet",
    "visor",
    "cage",
    "mask",
    "jersey",
    "sock",
    "pant",
    "skate",
    "boot",
    "font",
    "hat",
    "elbow",
    "shoulder",
    "shinpad",
    "shin",
    "girdle",
];

/// Body/suit prefixes for staff (coach/referee/trainer) — geometry, but not a head.
const BODY_KEYWORDS: &[&str] = &[
    "coachsuit",
    "trainersuit",
    "refereejersey",
    "refereepants",
    "refereeleftskate",
    "refereerightskate",
    "refereeleftwhistle",
    "refereerightwhistle",
    "suit",
    "body",
    "hand",
];

/// Submesh-name keywords that indicate distinct facial-hair geometry (rare — on
/// vanilla heads facial hair is painted into the texture, so a hit is notable).
const FACIAL_HAIR_KEYWORDS: &[&str] =
    &["beard", "moustache", "mustache", "goatee", "stubble", "facial", "sideburn"];

/// Is `stem` (filename minus extension) a texture raster? True when its final
/// `_`-segment is one of [`MAP_SUFFIXES`].
fn stem_is_texture(stem: &str) -> bool {
    stem.rsplit('_')
        .next()
        .is_some_and(|last| MAP_SUFFIXES.contains(&last))
}

/// The head id is the **first** numeric run in the name: `player_10043` → 10043,
/// `head_generic_player_60046` → 60046 (its only run), `hair_512_0_0_0` → 512 (the
/// trailing `_0_0_0` is the style/length/colour variant, **not** the id — taking
/// the last run would collapse every separate-hair file to id 0).
fn parse_id(stem: &str) -> Option<u64> {
    let mut cur = String::new();
    for ch in stem.chars() {
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            return cur.parse().ok();
        }
    }
    cur.parse().ok()
}

fn starts_with_any(name: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| name.starts_with(k))
}

/// Classify one file by its folder + name. Pure and filesystem-free, so it is unit
/// tested against synthetic names (no mount needed).
pub fn classify(folder: &str, file: &str, platform: Platform) -> CensusEntry {
    let lower = file.to_ascii_lowercase();
    let stem = Path::new(&lower)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&lower);
    let ext = Path::new(&lower)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    // `texlib_{id}.rx2` is a bundled multi-raster texture container — it carries no
    // map suffix, so match it by prefix or it reads as a bare-id head mesh.
    let is_texture = matches!(ext, "dds" | "gnf" | "png")
        || (ext == "rx2" && (stem.starts_with("texlib") || stem_is_texture(stem)));
    let is_generic = stem.contains("generic_player") || stem.starts_with("generic_");

    // Role from folder, refined by the generic marker.
    let folder_l = folder.to_ascii_lowercase();
    let mut role = match folder_l.as_str() {
        "player" | "skater" => Role::Player,
        "coach" | "coachhead" => Role::Coach,
        "referee" => Role::Referee,
        "trainer" => Role::Trainer,
        "commissioner" => Role::Commissioner,
        "timekeeper" => Role::Timekeeper,
        "czhead" | "czhair" => Role::Generic,
        _ => Role::Unknown,
    };
    // `generic_player_*` is a shared *player* pool head → Generic role. But
    // `generic_referee_*` / `generic_coach_*` / `generic_commissioner_*` are still
    // that staff role (just the shared variant), so only the player form remaps.
    if stem.contains("generic_player") {
        role = Role::Generic;
    }

    // Part from the name. Order matters: hair_ before head_ (so `hair_generic…`
    // isn't caught as head), equipment/body keywords before the bare-id head
    // fallback (so a PS4 `goaliehelmet_*` in player/ is gear, not a head).
    let part = if is_texture {
        Part::Texture
    } else if stem.starts_with("hair") {
        Part::Hair
    } else if stem.starts_with("eyel") || stem.starts_with("eyer") || stem.starts_with("eyes") {
        Part::Eye
    } else if stem.starts_with("head") || stem.starts_with("coachhead") {
        Part::Head
    } else if starts_with_any(stem, EQUIPMENT_KEYWORDS) {
        Part::Equipment
    } else if starts_with_any(stem, BODY_KEYWORDS) {
        Part::Body
    } else if matches!(folder_l.as_str(), "player" | "skater" | "coachhead")
        || (role == Role::Generic && stem.contains("generic_player"))
    {
        // Bare-id mesh in a head folder: RenderWare `player_{id}` / `coachhead_{id}`,
        // PS4 bespoke `{slug}.mcd`, or `generic_player_{id}.mcd` (base pool head).
        Part::Head
    } else if matches!(
        role,
        Role::Coach | Role::Referee | Role::Trainer | Role::Commissioner | Role::Timekeeper
    ) {
        // A bare-id mesh in a staff *body* folder (e.g. NHL 07 `referee_0`,
        // `coach_0`) — a full staff model; whether it carries a head submesh is
        // resolved by the sampled decode, not the name.
        Part::Body
    } else {
        Part::Unknown
    };

    let _ = platform; // reserved: PS4-specific tweaks key off folder already.

    CensusEntry {
        folder: folder.to_string(),
        file: file.to_string(),
        role,
        part,
        is_texture,
        is_generic,
        id: parse_id(stem),
        submeshes: Vec::new(),
        verts: None,
        facial_hair_geom: false,
        decode_error: None,
    }
}

/// Class label for the counts map, e.g. `player/head`, `generic/hair`, `coach/head`.
fn class_label(e: &CensusEntry) -> String {
    let role = match e.role {
        Role::Player => "player",
        Role::Generic => "generic",
        Role::Coach => "coach",
        Role::Referee => "referee",
        Role::Trainer => "trainer",
        Role::Commissioner => "commissioner",
        Role::Timekeeper => "timekeeper",
        Role::Unknown => "unknown",
    };
    let part = match e.part {
        Part::Head => "head",
        Part::Hair => "hair",
        Part::Eye => "eye",
        Part::Body => "body",
        Part::Equipment => "equipment",
        Part::Texture => "texture",
        Part::Unknown => "unknown",
    };
    format!("{role}/{part}")
}

/// The `rendering/` (RenderWare) or `characters/` (PS4) subfolders that can hold
/// head / hair / staff geometry. We walk exactly these — not the whole tree — so
/// the census stays scoped to appearance assets.
pub fn appearance_folders(platform: Platform) -> &'static [&'static str] {
    if platform == Platform::Ps4 {
        &["player", "skater", "coach", "referee", "trainer", "commissioner"]
    } else {
        &[
            "player",
            "coachhead",
            "coach",
            "referee",
            "trainer",
            "commissioner",
            "timekeeper",
            "czhead",
            "czhair",
        ]
    }
}

/// Deep-decode a mesh file for its submesh names + vertex total. Returns
/// `Err(message)` when the parser refuses it (recorded, not fatal).
fn decode_submeshes(path: &Path) -> std::result::Result<(Vec<String>, usize), String> {
    let bytes = crate::assets::read_asset_file(path).map_err(|e| e.to_string())?;
    let asset = crate::assets::parse_head_mesh(path, &bytes, true).map_err(|e| e.to_string())?;
    let names: Vec<String> = asset.submeshes.iter().map(|s| s.name.clone()).collect();
    let verts = asset.submeshes.iter().map(|s| s.vertices.len()).sum();
    Ok((names, verts))
}

/// Run the census over one title's tree. `sample_per_class` bounds how many meshes
/// per `role/part` class get deep-decoded for the submesh vocabulary (0 = filename
/// inventory only). The full filename classification always covers every file.
pub fn census_tree(
    rendering_root: &Path,
    platform: Platform,
    title: &str,
    sample_per_class: usize,
) -> CensusReport {
    let mut report = CensusReport {
        title: title.to_string(),
        platform: platform.as_str().to_string(),
        ..Default::default()
    };
    let base = if platform == Platform::Ps4 {
        rendering_root.join("characters")
    } else {
        rendering_root.to_path_buf()
    };

    let mut decoded_per_class: BTreeMap<String, usize> = BTreeMap::new();

    for folder in appearance_folders(platform) {
        let dir = base.join(folder);
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut names: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        names.sort();
        for path in names {
            // NHL 07 nests one folder per head (`player/player_{id}/head_{id}.rx2`).
            if path.is_dir() {
                if let Ok(inner) = std::fs::read_dir(&path) {
                    let mut inner: Vec<PathBuf> =
                        inner.filter_map(|e| e.ok().map(|e| e.path())).collect();
                    inner.sort();
                    for ip in inner {
                        classify_and_push(
                            &mut report,
                            &mut decoded_per_class,
                            folder,
                            &ip,
                            platform,
                            sample_per_class,
                        );
                    }
                }
                continue;
            }
            classify_and_push(
                &mut report,
                &mut decoded_per_class,
                folder,
                &path,
                platform,
                sample_per_class,
            );
        }
    }
    report
}

#[allow(clippy::too_many_arguments)]
fn classify_and_push(
    report: &mut CensusReport,
    decoded_per_class: &mut BTreeMap<String, usize>,
    folder: &str,
    path: &Path,
    platform: Platform,
    sample_per_class: usize,
) {
    let Some(file) = path.file_name().and_then(|s| s.to_str()) else {
        return;
    };
    // Only consider mesh/texture containers.
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "rx2" | "mcd" | "gnf" | "dds" | "png") {
        return;
    }
    let mut entry = classify(folder, file, platform);
    let label = class_label(&entry);
    *report.counts.entry(label.clone()).or_default() += 1;

    // Deep-decode a bounded sample of head/hair/body geometry per class.
    let want_decode = matches!(entry.part, Part::Head | Part::Hair | Part::Body)
        && !entry.is_texture
        && sample_per_class > 0;
    if want_decode {
        let n = decoded_per_class.entry(label).or_default();
        if *n < sample_per_class {
            *n += 1;
            match decode_submeshes(path) {
                Ok((names, verts)) => {
                    for nm in &names {
                        *report.submesh_vocab.entry(nm.to_ascii_lowercase()).or_default() += 1;
                        let low = nm.to_ascii_lowercase();
                        if FACIAL_HAIR_KEYWORDS.iter().any(|k| low.contains(k)) {
                            entry.facial_hair_geom = true;
                        }
                    }
                    if entry.facial_hair_geom {
                        report.facial_hair_hits.push(entry.file.clone());
                    }
                    entry.submeshes = names;
                    entry.verts = Some(verts);
                }
                Err(e) => entry.decode_error = Some(e),
            }
        }
    }
    report.entries.push(entry);
}

/// Serialize a report's per-file rows to CSV.
pub fn report_to_csv(report: &CensusReport) -> String {
    let mut out = String::from(
        "folder,file,role,part,is_texture,is_generic,id,verts,facial_hair_geom,submesh_count,decode_error\n",
    );
    for e in &report.entries {
        out.push_str(&format!(
            "{},{},{:?},{:?},{},{},{},{},{},{},{}\n",
            e.folder,
            e.file,
            e.role,
            e.part,
            e.is_texture,
            e.is_generic,
            e.id.map(|i| i.to_string()).unwrap_or_default(),
            e.verts.map(|v| v.to_string()).unwrap_or_default(),
            e.facial_hair_geom,
            e.submeshes.len(),
            e.decode_error.as_deref().unwrap_or("").replace(',', ";"),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_suffixes_are_detected() {
        assert!(stem_is_texture("haircp_1_0_cm"));
        assert!(stem_is_texture("coach_0_cam"));
        assert!(stem_is_texture("hand_cnm"));
        assert!(stem_is_texture("head_67_cm"));
        // Hair-card raster variants are textures, not meshes.
        assert!(stem_is_texture("hair_100_0_aa"));
        assert!(stem_is_texture("hair_100_0_ab"));
        // …but the hair mesh itself (`_0_0_0`) is not a texture.
        assert!(!stem_is_texture("hair_100_0_0_0"));
        assert!(!stem_is_texture("player_10043"));
        assert!(!stem_is_texture("coachsuit_0_0"));
        assert!(!stem_is_texture("alexander_ovechkin"));
    }

    #[test]
    fn ids_take_the_first_numeric_run() {
        assert_eq!(parse_id("player_10043"), Some(10043));
        assert_eq!(parse_id("head_generic_player_60046"), Some(60046));
        // The head id is the first run; the `_0_0_0` variant infix is not the id.
        assert_eq!(parse_id("hair_67_0_0_0"), Some(67));
        assert_eq!(parse_id("hair_512_0_0_0"), Some(512));
        assert_eq!(parse_id("alexander_ovechkin"), None);
    }

    #[test]
    fn renderware_player_heads_and_hair() {
        // Legacy bundle head.
        let e = classify("player", "player_10043.rx2", Platform::Xbox);
        assert_eq!((e.role, e.part), (Role::Player, Part::Head));
        assert!(!e.is_texture && !e.is_generic);
        assert_eq!(e.id, Some(10043));
        // NHL 09 loose head + separate hair.
        assert_eq!(classify("player", "head_512.rx2", Platform::Xbox).part, Part::Head);
        let hair = classify("player", "hair_512_0_0_0.rx2", Platform::Xbox);
        assert_eq!((hair.role, hair.part), (Role::Player, Part::Hair));
    }

    #[test]
    fn cz_folders_are_textures_not_heads() {
        let e = classify("czhead", "eyebrow_0_0_cm.rx2", Platform::Xbox);
        assert_eq!(e.part, Part::Texture);
        assert_eq!(e.role, Role::Generic);
        let h = classify("czhair", "haircp_1_0_cm.rx2", Platform::Xbox);
        assert_eq!(h.part, Part::Texture);
    }

    #[test]
    fn coachhead_heads_hair_eyes() {
        assert_eq!(classify("coachhead", "coachhead_1000.rx2", Platform::Xbox).part, Part::Head);
        assert_eq!(classify("coachhead", "head_100.rx2", Platform::Xbox).part, Part::Head);
        assert_eq!(classify("coachhead", "hair_100.rx2", Platform::Xbox).part, Part::Hair);
        assert_eq!(classify("coachhead", "eyel_100_am.rx2", Platform::Xbox).part, Part::Texture);
        assert_eq!(classify("coachhead", "eyel_100.rx2", Platform::Xbox).part, Part::Eye);
    }

    #[test]
    fn staff_bodies_are_body_not_head() {
        assert_eq!(classify("coach", "coachsuit_0_0.rx2", Platform::Xbox).part, Part::Body);
        assert_eq!(classify("referee", "refereejersey_0_0.rx2", Platform::Xbox).part, Part::Body);
        assert_eq!(classify("trainer", "trainersuit_0_0.rx2", Platform::Xbox).part, Part::Body);
        // NHL 07 single generic staff model — a bare id in a staff folder.
        assert_eq!(classify("referee", "referee_0.rx2", Platform::Xbox).part, Part::Body);
    }

    #[test]
    fn ps4_player_folder_discriminates_head_hair_equipment_generic() {
        // Bespoke head (pure slug).
        let o = classify("player", "alexander_ovechkin.mcd", Platform::Ps4);
        assert_eq!((o.role, o.part), (Role::Player, Part::Head));
        // Generic pool head + hair share the player folder.
        let gh = classify("player", "head_generic_player_60046.mcd", Platform::Ps4);
        assert_eq!((gh.role, gh.part, gh.is_generic), (Role::Generic, Part::Head, true));
        let ghair = classify("player", "hair_generic_player_60046.mcd", Platform::Ps4);
        assert_eq!((ghair.role, ghair.part), (Role::Generic, Part::Hair));
        // Bare generic base head.
        let gb = classify("player", "generic_player_183.mcd", Platform::Ps4);
        assert_eq!((gb.role, gb.part, gb.is_generic), (Role::Generic, Part::Head, true));
        // A shared referee model keeps its Referee role (not remapped to Generic)
        // and reads as a staff Body to decode/split, not Unknown.
        let gr = classify("referee", "generic_referee_31000.mcd", Platform::Ps4);
        assert_eq!((gr.role, gr.part, gr.is_generic), (Role::Referee, Part::Body, true));
        let gc = classify("commissioner", "generic_commissioner_30050.mcd", Platform::Ps4);
        assert_eq!((gc.role, gc.part), (Role::Commissioner, Part::Body));
        // Equipment sharing the player folder must NOT read as a head.
        for gear in [
            "goaliehelmet_itech_0.mcd",
            "glove_left_reebok_0.mcd",
            "trapper_right_vaughn_0.mcd",
            "brokestick_0.mcd",
            "blocker_left_vaughn_0.mcd",
        ] {
            let g = classify("player", gear, Platform::Ps4);
            assert_eq!(g.part, Part::Equipment, "{gear} must classify as equipment");
        }
    }

    // ---- Mount-gated live census runner (skips in CI / clean-room) ----
    //
    // Reads a manifest of `platform<TAB>title<TAB>rendering_root` lines from the
    // path in `QA_CENSUS_MANIFEST`, runs the census over each, and writes per-title
    // CSVs + a summary to `QA_CENSUS_OUT` (default: the manifest's dir). Deep-decode
    // sample size from `QA_CENSUS_SAMPLE` (default 8). Run with:
    //   QA_CENSUS_MANIFEST=... cargo test -p studio-domain head_census::tests::run_census -- --ignored --nocapture
    #[test]
    #[ignore = "needs mounted extracted trees; set QA_CENSUS_MANIFEST"]
    fn run_census() {
        let Some(manifest) = std::env::var_os("QA_CENSUS_MANIFEST") else {
            eprintln!("QA_CENSUS_MANIFEST unset — nothing to census");
            return;
        };
        let manifest = PathBuf::from(manifest);
        let text = std::fs::read_to_string(&manifest).expect("read manifest");
        let out_dir = std::env::var_os("QA_CENSUS_OUT")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.parent().unwrap().to_path_buf());
        std::fs::create_dir_all(&out_dir).ok();
        let sample: usize = std::env::var("QA_CENSUS_SAMPLE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8);

        let mut summary = String::from("platform,title,class,count\n");
        let mut vocab_all: BTreeMap<String, usize> = BTreeMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 3 {
                eprintln!("skip malformed manifest line: {line}");
                continue;
            }
            let platform = match cols[0].to_ascii_lowercase().as_str() {
                "ps4" => Platform::Ps4,
                "ps3" => Platform::Ps3,
                "recomp" => Platform::Recomp,
                _ => Platform::Xbox,
            };
            let title = cols[1];
            let root = Path::new(cols[2]);
            if !root.is_dir() {
                eprintln!("skip {title}: root not present ({})", root.display());
                continue;
            }
            eprintln!("\n=== census {title} ({}) — {} ===", platform.as_str(), root.display());
            let report = census_tree(root, platform, title, sample);
            for (k, v) in &report.counts {
                eprintln!("  {k:<22} {v}");
                summary.push_str(&format!("{},{},{},{}\n", platform.as_str(), title, k, v));
            }
            if !report.facial_hair_hits.is_empty() {
                eprintln!("  facial-hair GEOMETRY on: {:?}", report.facial_hair_hits);
            }
            for (k, v) in &report.submesh_vocab {
                *vocab_all.entry(k.clone()).or_default() += v;
            }
            let csv = report_to_csv(&report);
            let fname = format!("{}-{}.csv", platform.as_str(), title.replace(' ', "_"));
            std::fs::write(out_dir.join(&fname), csv).expect("write per-title csv");
        }
        std::fs::write(out_dir.join("_summary.csv"), &summary).expect("write summary");
        let mut vocab_csv = String::from("submesh_name,occurrences\n");
        for (k, v) in &vocab_all {
            vocab_csv.push_str(&format!("{k},{v}\n"));
        }
        std::fs::write(out_dir.join("_submesh_vocab.csv"), vocab_csv).expect("write vocab");
        eprintln!("\nwrote census CSVs to {}", out_dir.display());
        eprintln!("submesh vocabulary ({} distinct):", vocab_all.len());
        for (k, v) in &vocab_all {
            eprintln!("  {k:<28} {v}");
        }
    }
}
