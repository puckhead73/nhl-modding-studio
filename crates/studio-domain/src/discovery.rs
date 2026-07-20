//! Install discovery — turn a root folder the user points at into the concrete files this project
//! needs (the `nhlng.db`, its metadata, the localization db, and the available save containers).
//!
//! The user never sees title IDs: they pick a *game* and a *console*, point at a root folder
//! (RPCS3 root for PS3, the `default.xex` install dir for Xbox), and discovery walks the known
//! on-disk layout to resolve everything. Save *types* are classified by **content**
//! ([`tdb_savedata::SaveContainer`]), never by trusting folder names — so a folder named `ROSTER`
//! that actually holds a settings save is reported as settings, not roster.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use tdb_savedata::{ContainerVersion, SaveContainer};

use crate::Platform;

/// Bytes of NUL-padded ASCII magic at the head of a save container (matches `tdb_savedata`).
const MAGIC_LEN: usize = 16;

/// The per-title product codes used purely for auto-detection (never surfaced to the user).
pub struct TitleIds {
    /// PS3 NTSC-U product code (`BLUS…`).
    pub ps3_us: &'static str,
    /// PS3 PAL product code (`BLES…`).
    pub ps3_eu: &'static str,
    /// Xbox 360 title id (hex).
    pub xbox: &'static str,
}

/// Look up the product/title codes for a studio game id (`nhl07` … `nhllegacy`). Returns `None` for
/// titles outside the proven PS3/Xbox range (PS4 / Recomp use the legacy file-pick path).
///
/// NHL 07–09 are **Xbox 360 only** in this tool (their next-gen release was 360-native), so their PS3
/// product codes are empty — [`detect_ps3`] guards against that and is never reached for them anyway
/// (the catalog offers only Xbox). The Xbox title ids are read from each title's `default.xex`.
pub fn title_ids(game_id: &str) -> Option<TitleIds> {
    let ids = match game_id {
        "nhl07" => ("", "", "454107F3"),
        "nhl08" => ("", "", "45410820"),
        "nhl09" => ("", "", "45410872"),
        "nhl10" => ("BLUS30345", "BLES00567", "454108BA"),
        "nhl11" => ("BLUS30568", "BLES00984", "454108FD"),
        "nhl12" => ("BLUS30791", "BLES01357", "45410964"),
        "nhl13" => ("BLUS31003", "BLES01691", "4541099B"),
        "nhl14" => ("BLUS31179", "BLES01853", "454109BE"),
        "nhl15" => ("BLUS31436", "BLES02037", "454109DC"),
        "nhllegacy" => ("BLUS31450", "BLES02153", "454109EC"),
        _ => return None,
    };
    Some(TitleIds {
        ps3_us: ids.0,
        ps3_eu: ids.1,
        xbox: ids.2,
    })
}

/// The PS4 content ids (`CUSA…`) for a studio game id, used as a **fast path** to
/// locate a title's data/saves on PS4. Content ids are region-specific, so this
/// list can never be exhaustive — discovery also resolves the real id from each
/// install's `param.sfo` TITLE (see [`ps4_scan_cusa_by_year`]), which is
/// authoritative. A title can have several known variants; discovery tries each.
pub fn ps4_title_ids(game_id: &str) -> &'static [&'static str] {
    // Verified against real installs first, then other known regional variants.
    match game_id {
        "nhl15" => &["CUSA00561", "CUSA50061"],
        "nhl16" => &["CUSA01909"],
        "nhl17" => &["CUSA04326", "CUSA04330"],
        "nhl18" => &["CUSA07569", "CUSA07580"],
        "nhl19" => &["CUSA11116"],
        "nhl20" => &["CUSA15439", "CUSA15447"],
        "nhl21" => &["CUSA18182"],
        _ => &[],
    }
}

/// The NHL title year for a studio PS4 game id (`nhl17` → 17). Used to identify a
/// title's install folder by its `param.sfo` TITLE when the region-specific
/// content id isn't in the known list above.
fn ps4_title_year(game_id: &str) -> Option<u32> {
    game_id.strip_prefix("nhl").and_then(|y| y.parse().ok())
}

/// One discovered save container, classified by its content.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveSlot {
    /// Stable kind tag: `settings` | `roster` | `beagm` | `beapro` | `hockeycard` | `season` | `unknown`.
    pub kind: String,
    /// Absolute path to the container file (`SYS-DATA` on PS3; the single save file on Xbox).
    pub path: String,
    /// Container variant: `v1` (read-only) | `v2` | `v4`.
    pub container_version: String,
    /// Whether this container variant supports the write path (v2/v4 only).
    pub writable: bool,
}

/// The resolved database files (any may be missing if the layout is incomplete).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DbPaths {
    pub nhlng: Option<String>,
    pub localization: Option<String>,
    pub meta_xml: Option<String>,
}

/// The outcome of pointing discovery at a root folder.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallReport {
    pub game_id: String,
    pub platform: String,
    /// The folder the user pointed at (RPCS3 root for PS3, install dir for Xbox).
    pub picked_root: String,
    /// The resolved per-game install dir (where the profile sidecar lives); `None` if not found.
    pub install_dir: Option<String>,
    pub db_paths: DbPaths,
    pub saves: Vec<SaveSlot>,
    /// The bare minimum to mod: an `nhlng.db` **and** a roster save are both present.
    pub minimum_met: bool,
    /// The database isn't loose yet but `.big` archives are present — the install
    /// needs extraction (Xbox / Recomp / PS4 ship the db packed). When `true`,
    /// the UI offers extraction before enforcing the modding minimum.
    pub extract_needed: bool,
}

impl InstallReport {
    fn finish(mut self) -> Self {
        self.minimum_met = if self.platform == "ps4" {
            // PS4's editable db (nhl_game.db) + its metadata must resolve; the live roster save is
            // primary-but-optional (the user may only have a Profile save).
            self.db_paths.nhlng.is_some() && self.db_paths.meta_xml.is_some()
        } else {
            self.db_paths.nhlng.is_some() && self.saves.iter().any(|s| s.kind == "roster")
        };
        self
    }
}

/// Walk a root folder for everything a project needs. `save_path` points discovery at the roster/
/// profile save location for platforms whose emulator saves *aren't* in a consistent place
/// (Xbox / Recomp); it's ignored for PS3 and PS4, whose saves live under the emulator root. For
/// Xbox, a `None` `save_path` falls back to scanning the default Xenia content directory.
pub fn detect_install(
    game_id: &str,
    platform: Platform,
    root: &Path,
    save_path: Option<&Path>,
) -> InstallReport {
    let mut report = InstallReport {
        game_id: game_id.to_string(),
        platform: platform.as_str().to_string(),
        picked_root: root.to_string_lossy().to_string(),
        install_dir: None,
        db_paths: DbPaths::default(),
        saves: Vec::new(),
        minimum_met: false,
        extract_needed: false,
    };

    match platform {
        Platform::Ps3 => {
            if let Some(ids) = title_ids(game_id) {
                detect_ps3(root, &ids, &mut report);
            }
        }
        Platform::Xbox => {
            if let Some(ids) = title_ids(game_id) {
                detect_xbox(root, &ids, save_path, &mut report);
            }
        }
        Platform::Ps4 => detect_ps4(root, game_id, ps4_title_ids(game_id), &mut report),
        // Recomp (Xbox-derived): a game folder (packed like Xbox) plus an explicit save path.
        Platform::Recomp => detect_recomp(root, save_path, &mut report),
    }

    report.finish()
}

/// Set `extract_needed` when a required db isn't loose yet but `.big` archives are present in
/// `install_dir` — i.e. the user pointed at a valid but un-extracted install. This covers two shapes:
/// the whole db is packed (Xbox nhl10+, Recomp), and — on nhl08/09 — the main `nhlng.db` ships loose
/// but the localization `nhl_eng_us.db` lives inside a `.big` (`fe/loc/`), so extraction is still
/// needed to resolve human-readable strings.
fn note_extract_needed(install_dir: &Path, report: &mut InstallReport) {
    let missing_db =
        report.db_paths.nhlng.is_none() || report.db_paths.localization.is_none();
    if missing_db && !crate::extract::find_big_archives(install_dir).is_empty() {
        report.extract_needed = true;
    }
}

fn detect_ps3(root: &Path, ids: &TitleIds, report: &mut InstallReport) {
    // Xbox-only titles (NHL 07–09) carry empty PS3 codes; nothing to detect and an empty prefix
    // would match every save folder in the scan below, so bail early.
    if ids.ps3_us.is_empty() && ids.ps3_eu.is_empty() {
        return;
    }
    // Databases: <root>/dev_hdd0/game/<TITLEID>/USRDIR (US then EU).
    let game_dir = root.join("dev_hdd0").join("game");
    let install_dir = [ids.ps3_us, ids.ps3_eu]
        .iter()
        .map(|tid| game_dir.join(tid).join("USRDIR"))
        .find(|p| p.is_dir());
    if let Some(dir) = &install_dir {
        report.install_dir = Some(dir.to_string_lossy().to_string());
        report.db_paths = resolve_db_paths(dir);
    }

    // Saves: <root>/dev_hdd0/home/00000001/savedata/<TITLEID…>/SYS-DATA.
    let savedata = root
        .join("dev_hdd0")
        .join("home")
        .join("00000001")
        .join("savedata");
    if let Ok(entries) = std::fs::read_dir(&savedata) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_uppercase();
            if !(name.starts_with(ids.ps3_us) || name.starts_with(ids.ps3_eu)) {
                continue;
            }
            if let Some(slot) = classify_save(&entry.path().join("SYS-DATA")) {
                report.saves.push(slot);
            }
        }
    }
}

fn detect_xbox(
    root: &Path,
    ids: &TitleIds,
    save_path: Option<&Path>,
    report: &mut InstallReport,
) {
    // The Xbox install dir *is* the folder the user pointed at (contains default.xex).
    report.install_dir = Some(root.to_string_lossy().to_string());
    report.db_paths = resolve_db_paths(root);
    note_extract_needed(root, report);

    match save_path {
        // The user pointed at their save location directly (Xenia's save location isn't
        // consistent) — scan it for containers.
        Some(sp) => report.saves = scan_saves(sp),
        // Fall back to the default Xenia content tree, title-filtered:
        // <xenia content>/<USER16HASH>/<TITLEID>/00000001/<SAVE>/<file>.
        None => {
            let Some(content_root) = default_xenia_content() else {
                return;
            };
            let Ok(hashes) = std::fs::read_dir(&content_root) else {
                return;
            };
            for hash in hashes.flatten() {
                let Some(title_dir) = child_ci(&hash.path(), ids.xbox) else {
                    continue;
                };
                let user_dir = title_dir.join("00000001");
                let Ok(saves) = std::fs::read_dir(&user_dir) else {
                    continue;
                };
                for save in saves.flatten() {
                    if !save.path().is_dir() {
                        continue;
                    }
                    // Xbox stores the container as a single file inside the save folder; pick the
                    // first file that parses as a save container.
                    if let Some(slot) = first_container_in(&save.path()) {
                        report.saves.push(slot);
                    }
                }
            }
        }
    }
}

/// Recomp (the NHL Legacy Recompilation) is Xbox-derived: the game folder is packed the same way
/// (so the db may need `.big` extraction) and uses the same big-endian TDB + save containers. Its
/// saves have no consistent emulator location, so the user provides the save path directly.
fn detect_recomp(root: &Path, save_path: Option<&Path>, report: &mut InstallReport) {
    report.install_dir = Some(root.to_string_lossy().to_string());
    report.db_paths = resolve_db_paths(root);
    note_extract_needed(root, report);
    if let Some(sp) = save_path {
        report.saves = scan_saves(sp);
    }
}

/// Depth bound for the save-folder scan — save trees are shallow; the cap guards against a user
/// pointing at an enormous directory.
const SAVE_WALK_DEPTH: usize = 6;

/// Discover save containers under an explicit save path (Xbox / Recomp). Accepts a single container
/// file *or* a folder: for a folder, walk it (bounded) and take the first recognised container in
/// each directory — mirroring the "one container per save folder" layout consoles use. Files that
/// don't classify to a known save type are ignored.
fn scan_saves(path: &Path) -> Vec<SaveSlot> {
    if path.is_file() {
        return classify_save(path).into_iter().collect();
    }
    let mut slots = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![(path.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if let Some(slot) = first_container_in(&dir) {
            if seen.insert(slot.path.clone()) {
                slots.push(slot);
            }
        }
        if depth < SAVE_WALK_DEPTH {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for e in entries.flatten() {
                    if e.path().is_dir() {
                        stack.push((e.path(), depth + 1));
                    }
                }
            }
        }
    }
    slots.sort_by(|a, b| a.path.cmp(&b.path));
    slots
}

/// shadPS4 layout: game files live under `<root>/Games/<CUSA>/nhl/data/...` (the `nhl_game.db` +
/// metadata + localization, once the big files are extracted) and saves under
/// `<root>/user/savedata/<user>/<CUSA>/<SaveDir>/DATA`. The user points at the shadPS4 folder (or the
/// savedata folder); we resolve both trees by the title's `CUSA…` content id.
///
/// Content ids are region-specific, so the hardcoded `known` list is only a fast
/// path. We also scan `<root>/Games/*` for the CUSA whose `param.sfo` TITLE year
/// matches this title (authoritative), and fold that into the candidate set —
/// this is what lets NHL 17+ resolve on installs whose real id differs from the
/// recorded one. The id discovered from the game tree is reused for savedata.
fn detect_ps4(root: &Path, game_id: &str, known: &[&str], report: &mut InstallReport) {
    let mut candidates: Vec<String> = known.iter().map(|s| s.to_string()).collect();
    if let Some(year) = ps4_title_year(game_id) {
        for cusa in ps4_scan_cusa_by_year(root, year) {
            if !candidates.iter().any(|c| c.eq_ignore_ascii_case(&cusa)) {
                candidates.push(cusa);
            }
        }
    }
    let cusas: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();

    // --- Game files (need the big files extracted) — first CUSA variant that resolves a game dir ---
    if let Some(game_dir) = cusas.iter().find_map(|c| ps4_game_dir(root, c)) {
        // Fast path: the standard t3db / loc locations; fall back to a bounded recursive search.
        let t3db = game_dir.join("nhl/data/common/gamemodes/database/t3db");
        let nhlng = some_file(t3db.join("nhl_game.db"))
            .or_else(|| find_file(&game_dir, "nhl_game.db", 9));
        if let Some(db) = nhlng {
            report.install_dir = db.parent().map(|d| d.to_string_lossy().into_owned());
            report.db_paths.meta_xml = db
                .parent()
                .map(|d| d.join("nhl_game-meta.xml"))
                .filter(|p| p.exists())
                .map(path_string);
            report.db_paths.nhlng = Some(path_string(db));
        }
        report.db_paths.localization = some_file(game_dir.join("nhl/data/kettle/loc/eng_us.db"))
            .or_else(|| find_file(&game_dir, "eng_us.db", 9))
            .map(path_string);
        note_extract_needed(&game_dir, report);
        // PS4 ships its databases *loose* but keeps the art (meshes / textures / portraits)
        // packed inside the databigs, so a resolved db is **not** proof the install is fully
        // unpacked — unlike Xbox, where the db and art come out of the same `.big` set. Flag
        // extraction whenever the kettle art tree is still missing but `.big` archives are
        // present, so asset editing has files to work with. (Extraction leaves the `.big`
        // files in place, so `complete()` — not their absence — is the "already unpacked" signal.)
        if !report.extract_needed
            && !crate::assets::AssetRoots::resolve(&game_dir, Platform::Ps4).complete()
            && !crate::extract::find_big_archives(&game_dir).is_empty()
        {
            report.extract_needed = true;
        }
    }

    // --- Saves: each <SaveDir>/DATA under the title's savedata CUSA folder (first variant found) ---
    if let Some(saves_dir) = cusas.iter().find_map(|c| ps4_savedata_cusa_dir(root, c)) {
        if let Ok(entries) = std::fs::read_dir(&saves_dir) {
            let mut dirs: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            dirs.sort();
            for dir in dirs {
                let data = dir.join("DATA");
                if !data.is_file() {
                    continue;
                }
                // A save is "writable" here if our codec recognises its inner LZ4 roster TDB.
                let writable = std::fs::read(&data)
                    .map(|b| tdb_savedata::is_ps4_save(&b))
                    .unwrap_or(false);
                let name = dir.file_name().unwrap_or_default().to_string_lossy();
                report.saves.push(SaveSlot {
                    kind: ps4_save_kind(&name).to_string(),
                    path: path_string(data),
                    container_version: "ps4".to_string(),
                    writable,
                });
            }
        }
    }
}

/// The title's game-files dir: `<root>/Games/<CUSA>` (shadPS4 root) or `<root>/<CUSA>`.
fn ps4_game_dir(root: &Path, cusa: &str) -> Option<PathBuf> {
    child_ci(root, "Games")
        .and_then(|g| child_ci(&g, cusa))
        .or_else(|| child_ci(root, cusa).filter(|c| c.join("nhl").is_dir()))
}

/// Scan `<root>/Games/*` (and `<root>/*`, for a directly-picked Games folder) for
/// `CUSA…` install dirs whose `param.sfo` TITLE year matches `year`, returning
/// their content-id folder names. This is the authoritative title→id resolution:
/// content ids are region-specific and can't be exhaustively hardcoded, but every
/// install carries its title in `sce_sys/param.sfo`.
fn ps4_scan_cusa_by_year(root: &Path, year: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Some(games) = child_ci(root, "Games") {
        bases.push(games);
    }
    bases.push(root.to_path_buf());
    for base in bases {
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.to_ascii_uppercase().starts_with("CUSA") {
                continue;
            }
            if ps4_param_sfo_year(&path) == Some(year)
                && !out.iter().any(|c| c.eq_ignore_ascii_case(&name))
            {
                out.push(name);
            }
        }
    }
    out
}

/// Read the NHL title year from a PS4 install's `sce_sys/param.sfo` TITLE
/// (e.g. `"NHL® 17"` → `17`). Returns `None` if the file/field is absent or has
/// no trailing number.
fn ps4_param_sfo_year(cusa_dir: &Path) -> Option<u32> {
    let sfo = child_ci(cusa_dir, "sce_sys")?.join("param.sfo");
    let data = std::fs::read(sfo).ok()?;
    let title = sfo_string_value(&data, "TITLE")?;
    // The title reads "NHL® 15".."NHL® 24" (the ® is mojibake in some regions);
    // take the last run of digits as the year.
    let mut year: Option<u32> = None;
    let mut cur = String::new();
    for ch in title.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            year = cur.parse().ok();
            cur.clear();
        }
    }
    year
}

/// Extract a string value for `key` from a Sony `param.sfo` (the PSF key/value
/// blob). Minimal reader: enough to pull `TITLE`/`TITLE_ID`. Returns `None` on
/// any malformation rather than panicking on untrusted input.
fn sfo_string_value(data: &[u8], key: &str) -> Option<String> {
    if data.len() < 20 || &data[0..4] != b"\x00PSF" {
        return None;
    }
    let u32_at = |o: usize| -> Option<usize> {
        data.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let key_table = u32_at(0x08)?;
    let data_table = u32_at(0x0C)?;
    let count = u32_at(0x10)?;
    for i in 0..count {
        let e = 0x14 + i * 0x10;
        let key_off = data.get(e..e + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)?;
        let len = u32_at(e + 4)?;
        let data_off = u32_at(e + 0x0C)?;
        // Key: NUL-terminated at key_table + key_off.
        let kstart = key_table + key_off;
        let kend = data[kstart..].iter().position(|&b| b == 0).map(|p| kstart + p)?;
        let k = std::str::from_utf8(&data[kstart..kend]).ok()?;
        if k != key {
            continue;
        }
        let vstart = data_table + data_off;
        let vslice = data.get(vstart..vstart + len)?;
        let vend = vslice.iter().position(|&b| b == 0).unwrap_or(vslice.len());
        return Some(String::from_utf8_lossy(&vslice[..vend]).into_owned());
    }
    None
}

/// The title's savedata dir holding its save folders: handles the user pointing at the shadPS4 root
/// (`user/savedata/<user>/<CUSA>`) or directly at a savedata user folder (`<root>/<CUSA>`).
fn ps4_savedata_cusa_dir(root: &Path, cusa: &str) -> Option<PathBuf> {
    let mut bases: Vec<PathBuf> = vec![root.to_path_buf()];
    let savedata = child_ci(root, "user")
        .and_then(|u| child_ci(&u, "savedata"))
        .or_else(|| child_ci(root, "savedata"));
    if let Some(sd) = savedata {
        if let Ok(entries) = std::fs::read_dir(&sd) {
            bases.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
        }
    }
    bases
        .into_iter()
        .find_map(|base| child_ci(&base, cusa).filter(|c| has_save_with_data(c)))
}

/// `true` if `cusa_dir` has at least one subfolder containing a `DATA` file (a PS4 save).
fn has_save_with_data(cusa_dir: &Path) -> bool {
    std::fs::read_dir(cusa_dir)
        .map(|it| {
            it.flatten()
                .any(|e| e.path().is_dir() && e.path().join("DATA").is_file())
        })
        .unwrap_or(false)
}

/// Classify a PS4 save by its Sony export folder name (e.g. `RosterCustom0`, `Profile0`, `Season0`).
fn ps4_save_kind(dir_name: &str) -> &'static str {
    let n = dir_name.to_ascii_lowercase();
    if n.contains("roster") {
        "roster"
    } else if n.contains("profile") {
        "settings"
    } else if n.contains("season") {
        "season"
    } else if n.contains("beagm") {
        "beagm"
    } else if n.contains("beapro") {
        "beapro"
    } else {
        "unknown"
    }
}

fn some_file(p: PathBuf) -> Option<PathBuf> {
    p.is_file().then_some(p)
}

/// Resolve the three database files within an install dir via a bounded recursive search (the exact
/// subfolder layout varies, so we match by filename rather than assume a fixed depth).
fn resolve_db_paths(dir: &Path) -> DbPaths {
    // The editable main db is `nhlng.db` on nhl08+, but nhl07 ships it as `NHL07NG.db` (with a
    // matching `NHL07NG-meta.xml`). Try each name in order; no install carries both.
    DbPaths {
        nhlng: find_first_file(dir, &["nhlng.db", "NHL07NG.db"], 6).map(path_string),
        localization: find_file(dir, "nhl_eng_us.db", 6).map(path_string),
        meta_xml: find_first_file(dir, &["nhlng-meta.xml", "NHL07NG-meta.xml"], 6).map(path_string),
    }
}

/// First file under `root` matching any of `names` (case-insensitive), tried in order.
fn find_first_file(root: &Path, names: &[&str], max_depth: usize) -> Option<PathBuf> {
    names.iter().find_map(|n| find_file(root, n, max_depth))
}

/// First file under `root` whose final component equals `name` (case-insensitive), up to `max_depth`.
fn find_file(root: &Path, name: &str, max_depth: usize) -> Option<PathBuf> {
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                if depth < max_depth {
                    stack.push((entry.path(), depth + 1));
                }
            } else if entry.file_name().to_string_lossy().eq_ignore_ascii_case(name) {
                return Some(entry.path());
            }
        }
    }
    None
}

/// The first regular file in `dir` that classifies as a *recognised* save container (skips sidecar
/// files like `PARAM.SFO` / `ICON0.PNG`, whose magic isn't a known save type).
fn first_container_in(dir: &Path) -> Option<SaveSlot> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    files.sort();
    files
        .iter()
        .filter_map(|p| classify_save(p))
        .find(|s| s.kind != "unknown")
}

/// Read and classify one save container. The save *type* comes from the magic string (readable on
/// every container version), so a save still lists even when its container version isn't yet
/// supported by the codec (e.g. the v5 UserProfile/Settings save) — it's just marked non-writable.
fn classify_save(path: &Path) -> Option<SaveSlot> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 0x18 {
        return None;
    }
    let kind = kind_from_magic(&bytes);
    // Container version + writability are best-effort: the codec handles v1/v2/v4; anything it
    // rejects (e.g. v5) is still surfaced, just flagged non-writable so the UI can gate editing.
    let (container_version, writable) = match SaveContainer::parse(&bytes) {
        Ok(c) => (version_str(c.header.version).to_string(), c.writeable()),
        Err(_) => {
            let vw = u32::from_be_bytes([bytes[0x14], bytes[0x15], bytes[0x16], bytes[0x17]]);
            (format!("v{vw}"), false)
        }
    };
    Some(SaveSlot {
        kind,
        path: path_string(path.to_path_buf()),
        container_version,
        writable,
    })
}

/// Classify a save's type from its container magic. Tolerates the optional `PS3` prefix and the
/// 16-byte magic truncation (e.g. `PS3UserProfileFile` is cut to `PS3UserProfileFi`), so we match by
/// the type token the magic *contains* rather than an exact string.
fn kind_from_magic(bytes: &[u8]) -> String {
    let cap = MAGIC_LEN.min(bytes.len());
    let end = bytes[..cap].iter().position(|&b| b == 0).unwrap_or(cap);
    let magic = String::from_utf8_lossy(&bytes[..end]);
    let m = magic.as_ref();
    let kind = if m.contains("Roster") {
        "roster"
    } else if m.contains("UserProfile") {
        "settings"
    } else if m.contains("Dynasty") {
        "beagm"
    } else if m.contains("BeAPro") {
        "beapro"
    } else if m.contains("HockeyCard") {
        "hockeycard"
    } else if m.contains("Season") {
        "season"
    } else {
        "unknown"
    };
    kind.to_string()
}

/// Find a direct child directory of `parent` whose name matches `name` case-insensitively.
fn child_ci(parent: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(parent).ok()?;
    entries.flatten().find_map(|e| {
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir && e.file_name().to_string_lossy().eq_ignore_ascii_case(name) {
            Some(e.path())
        } else {
            None
        }
    })
}

/// The default Xenia content directory: `%USERPROFILE%/Documents/Xenia/content`.
fn default_xenia_content() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE")?;
    Some(
        PathBuf::from(home)
            .join("Documents")
            .join("Xenia")
            .join("content"),
    )
}

fn version_str(v: ContainerVersion) -> &'static str {
    match v {
        ContainerVersion::RawV1 => "v1",
        ContainerVersion::RawV2 => "v2",
        ContainerVersion::ZlibV4 => "v4",
    }
}

fn path_string(p: PathBuf) -> String {
    p.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// The committed anchors, shaped like a real install in tests below.
    fn anchors() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors")
    }

    /// A unique temp dir, cleaned up on drop.
    struct TempTree(PathBuf);
    impl TempTree {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("nhlms-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempTree(dir)
        }
    }
    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn copy(from: PathBuf, to: PathBuf) {
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(&from, &to).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", from.display(), to.display()));
    }

    #[test]
    fn ps3_detect_resolves_modding_minimum() {
        let a = anchors().join("ps3/nhl14");
        let t = TempTree::new("ps3");
        let root = &t.0;
        let usrdir = root.join("dev_hdd0/game/BLUS31179/USRDIR");
        copy(a.join("databases/nhlng.db"), usrdir.join("nhlng.db"));
        copy(a.join("databases/nhlng-meta.xml"), usrdir.join("nhlng-meta.xml"));
        copy(a.join("databases/nhl_eng_us.db"), usrdir.join("nhl_eng_us.db"));
        let sd = root.join("dev_hdd0/home/00000001/savedata");
        copy(
            a.join("savefiles/BLUS311790200 - ROSTER/SYS-DATA"),
            sd.join("BLUS311790200 - ROSTER/SYS-DATA"),
        );
        copy(
            a.join("savefiles/BLUS311790000 - SETTINGS/SYS-DATA"),
            sd.join("BLUS311790000 - SETTINGS/SYS-DATA"),
        );

        let r = detect_install("nhl14", Platform::Ps3, root, None);
        assert!(r.minimum_met, "nhlng + roster present => minimum met");
        assert!(r.db_paths.nhlng.is_some());
        assert!(r.db_paths.meta_xml.is_some());
        assert!(r.db_paths.localization.is_some());
        let roster = r.saves.iter().find(|s| s.kind == "roster").unwrap();
        assert!(roster.writable, "v4 roster is writable");
        // The Settings save is a v5 container the codec can't write yet: listed, but non-writable.
        let settings = r.saves.iter().find(|s| s.kind == "settings").unwrap();
        assert_eq!(settings.container_version, "v5");
        assert!(!settings.writable);
        assert!(r.install_dir.is_some());
    }

    #[test]
    fn xbox_detect_classifies_profile_as_settings() {
        let a = anchors().join("xbox360/nhl14");
        let t = TempTree::new("xbox");
        let install = t.0.join("install");
        copy(a.join("databases/nhlng.db"), install.join("nhlng.db"));
        copy(a.join("databases/nhlng-meta.xml"), install.join("nhlng-meta.xml"));
        fs::write(install.join("default.xex"), b"xex").unwrap();

        // Xenia content: <content>/<USER16HASH>/<TITLEID>/00000001/<SAVE>/<file>.
        let content = t.0.join("content");
        let user = content.join("ABCDEF0123456789/454109BE/00000001");
        copy(
            a.join("savefiles/ROSTER 20260627054746/ROSTER 20260627054746"),
            user.join("ROSTER 20260627054746/ROSTER 20260627054746"),
        );
        copy(
            a.join("savefiles/PROFILE 20260627054703/PROFILE 20260627054703"),
            user.join("PROFILE 20260627054703/PROFILE 20260627054703"),
        );

        let r = detect_install("nhl14", Platform::Xbox, &install, Some(&content));
        assert!(r.minimum_met, "nhlng + roster present => minimum met");
        assert!(r.saves.iter().any(|s| s.kind == "roster"));
        // The Xbox PROFILE save carries the settings (magic `UserProfileFile`).
        assert!(
            r.saves.iter().any(|s| s.kind == "settings"),
            "xbox PROFILE save classifies as settings"
        );
    }

    #[test]
    fn detect_without_game_install_is_not_minimum() {
        let t = TempTree::new("empty");
        let r = detect_install("nhl14", Platform::Ps3, &t.0, None);
        assert!(!r.minimum_met);
        assert!(r.db_paths.nhlng.is_none());
        assert!(r.saves.is_empty());
    }

    /// Write a minimal `.big` archive containing the given entries.
    fn write_big(path: &Path, entries: &[(&str, &[u8])]) {
        let mut b = tdb_archive_big::ArchiveBuilder::new();
        for (name, body) in entries {
            b.add_entry(*name, body.to_vec());
        }
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b.build().unwrap()).unwrap();
    }

    #[test]
    fn xbox_unextracted_install_reports_extract_needed_not_minimum() {
        // A fresh Xbox install: default.xex + a packed .big, but no loose nhlng.db.
        let t = TempTree::new("xbox-unextracted");
        let install = t.0.join("install");
        fs::create_dir_all(&install).unwrap();
        fs::write(install.join("default.xex"), b"xex").unwrap();
        write_big(&install.join("data.big"), &[("db/nhlng.db", b"packed")]);

        let r = detect_install("nhl14", Platform::Xbox, &install, None);
        assert!(r.extract_needed, "packed .big + no loose db => extraction needed");
        assert!(!r.minimum_met, "can't mod until the db is extracted");
        assert!(r.db_paths.nhlng.is_none());
    }

    #[test]
    fn recomp_detects_game_db_and_scanned_save() {
        // Recomp uses the Xbox big-endian db + save containers. Point at a game folder holding a
        // loose nhlng.db + meta, and a separate save folder with a real roster container.
        let a = anchors().join("xbox360/nhl14");
        let t = TempTree::new("recomp");
        let game = t.0.join("game");
        copy(a.join("databases/nhlng.db"), game.join("nhlng.db"));
        copy(a.join("databases/nhlng-meta.xml"), game.join("nhlng-meta.xml"));

        let saves = t.0.join("saves");
        copy(
            a.join("savefiles/ROSTER 20260627054746/ROSTER 20260627054746"),
            saves.join("ROSTER/container"),
        );

        let r = detect_install("legacyrecomp", Platform::Recomp, &game, Some(&saves));
        assert!(r.db_paths.nhlng.is_some(), "recomp nhlng.db resolved");
        assert!(r.saves.iter().any(|s| s.kind == "roster"), "roster save scanned");
        assert!(r.minimum_met, "nhlng + roster => recomp minimum met");
    }

    fn write_dummy(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    /// Build a shadPS4-shaped tree for NHL16 (CUSA01909): game files under `Games/<CUSA>/nhl/data/...`
    /// and a real roster save under `user/savedata/1/<CUSA>/RosterCustom0/DATA`. Discovery only
    /// resolves *paths* for the dbs, so those can be dummy files; the save DATA is the real anchor (the
    /// writability check parses it).
    fn build_shadps4_tree(root: &Path) {
        let cusa = "CUSA01909";
        let t3db = root.join("Games").join(cusa).join("nhl/data/common/gamemodes/database/t3db");
        write_dummy(&t3db.join("nhl_game.db"));
        write_dummy(&t3db.join("nhl_game-meta.xml"));
        write_dummy(&root.join("Games").join(cusa).join("nhl/data/kettle/loc/eng_us.db"));
        let roster = anchors().join("ps4/nhl16/savefiles/84f8102f - ROSTER/DATA");
        copy(
            roster,
            root.join("user/savedata/1").join(cusa).join("RosterCustom0/DATA"),
        );
    }

    #[test]
    fn ps4_shadps4_root_resolves_game_db_meta_and_roster_save() {
        let t = TempTree::new("ps4root");
        build_shadps4_tree(&t.0);

        let r = detect_install("nhl16", Platform::Ps4, &t.0, None);
        assert!(r.db_paths.nhlng.is_some(), "nhl_game.db resolved");
        assert!(r.db_paths.meta_xml.is_some(), "nhl_game-meta.xml resolved");
        assert!(r.db_paths.localization.is_some(), "eng_us.db resolved");
        assert!(r.minimum_met, "PS4 minimum = nhl_game.db + meta");
        let roster = r.saves.iter().find(|s| s.kind == "roster").expect("RosterCustom save");
        assert_eq!(roster.container_version, "ps4");
        assert!(roster.writable, "real NHL16 roster DATA is recognised by the PS4 codec");
    }

    #[test]
    fn ps4_loose_db_but_packed_art_flags_extract_needed() {
        // PS4 ships the db loose, so the modding minimum is met immediately — but the art is
        // still inside a databig and no kettle rendering tree exists. Extraction must still be
        // offered (the Assets/Export pages have nothing to scan until it runs).
        let t = TempTree::new("ps4-packed-art");
        build_shadps4_tree(&t.0);
        let game = t.0.join("Games").join("CUSA01909");
        write_big(&game.join("databigs/inxp_00.big"), &[("nhl/data/kettle/x.gnf", b"packed")]);

        let r = detect_install("nhl16", Platform::Ps4, &t.0, None);
        assert!(r.db_paths.nhlng.is_some(), "loose nhl_game.db still resolves");
        assert!(r.minimum_met, "PS4 minimum met from the loose db alone");
        assert!(
            r.extract_needed,
            "loose db + packed art (no kettle rendering tree) => extraction still needed"
        );
    }

    #[test]
    fn ps4_extracted_art_does_not_reflag_extract_needed() {
        // A databig stays on disk after extraction, so its presence alone must NOT re-flag the
        // install — the extracted kettle player-art tree is the "already unpacked" signal.
        let t = TempTree::new("ps4-extracted-art");
        build_shadps4_tree(&t.0);
        let game = t.0.join("Games").join("CUSA01909");
        write_big(&game.join("databigs/inxp_00.big"), &[("nhl/data/kettle/x.gnf", b"packed")]);
        // The extracted player character-art dir `complete()` checks for.
        fs::create_dir_all(game.join("nhl/data/kettle/rendering/characters/player")).unwrap();

        let r = detect_install("nhl16", Platform::Ps4, &t.0, None);
        assert!(
            !r.extract_needed,
            "art already extracted => no extraction prompt despite the .big still present"
        );
    }

    #[test]
    fn ps4_pointing_at_savedata_only_finds_saves_but_not_minimum() {
        let t = TempTree::new("ps4save");
        build_shadps4_tree(&t.0);

        // Point directly at the savedata user folder (no game files reachable).
        let r = detect_install("nhl16", Platform::Ps4, &t.0.join("user/savedata/1"), None);
        assert!(r.saves.iter().any(|s| s.kind == "roster"), "save still found");
        assert!(r.db_paths.nhlng.is_none(), "no game db reachable from savedata alone");
        assert!(!r.minimum_met, "without nhl_game.db + meta, PS4 minimum is unmet");
    }

    /// Build a minimal Sony `param.sfo` blob carrying a single `TITLE` value.
    fn build_sfo(title: &str) -> Vec<u8> {
        let key = b"TITLE\0";
        let val = {
            let mut v = title.as_bytes().to_vec();
            v.push(0);
            v
        };
        let index_end = 0x14 + 0x10; // header + one 16-byte index entry
        let key_table = index_end;
        let data_table = key_table + key.len();
        let mut d = Vec::new();
        d.extend_from_slice(b"\x00PSF");
        d.extend_from_slice(&0x0101_0000u32.to_le_bytes()); // version
        d.extend_from_slice(&(key_table as u32).to_le_bytes());
        d.extend_from_slice(&(data_table as u32).to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes()); // one entry
        // index entry: key_offset, fmt(utf8=0x0204), len, max_len, data_offset
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&0x0204u16.to_le_bytes());
        d.extend_from_slice(&(val.len() as u32).to_le_bytes());
        d.extend_from_slice(&(val.len() as u32).to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(key);
        d.extend_from_slice(&val);
        d
    }

    #[test]
    fn sfo_year_extraction() {
        // Real installs: NHL 15/16/21 mojibake the ®, NHL 17/18/24 keep it.
        assert_eq!(ps4_title_year("nhl17"), Some(17));
        for (title, year) in [
            ("NHL\u{fffd}\u{84}\u{fffd} 15", 15),
            ("NHL® 17", 17),
            ("NHL® 24", 24),
        ] {
            let sfo = build_sfo(title);
            assert_eq!(
                sfo_string_value(&sfo, "TITLE").as_deref(),
                Some(title),
                "TITLE round-trips"
            );
        }
    }

    /// The regression for "games post NHL 17 not detected": the install's real
    /// content id differs from every hardcoded id, so resolution must fall back to
    /// the `param.sfo` TITLE. Here NHL 17 lives under a CUSA id that is *not* in
    /// `ps4_title_ids("nhl17")`, and detection must still find it.
    #[test]
    fn ps4_resolves_unknown_cusa_by_param_sfo() {
        let t = TempTree::new("ps4sfo");
        let cusa = "CUSA99999"; // deliberately not in the known list
        assert!(!ps4_title_ids("nhl17").contains(&cusa));
        let game = t.0.join("Games").join(cusa);
        // Identify the title only via param.sfo.
        write_dummy(&game.join("sce_sys/param.sfo"));
        fs::write(game.join("sce_sys/param.sfo"), build_sfo("NHL® 17")).unwrap();
        let t3db = game.join("nhl/data/common/gamemodes/database/t3db");
        write_dummy(&t3db.join("nhl_game.db"));
        write_dummy(&t3db.join("nhl_game-meta.xml"));

        let r = detect_install("nhl17", Platform::Ps4, &t.0, None);
        assert!(
            r.db_paths.nhlng.is_some(),
            "NHL17 game db resolved via param.sfo despite unknown CUSA id"
        );
        assert!(r.db_paths.meta_xml.is_some(), "meta resolved");
        assert!(r.minimum_met, "PS4 minimum met for the param.sfo-resolved install");

        // A different title must NOT claim this install.
        let other = detect_install("nhl18", Platform::Ps4, &t.0, None);
        assert!(other.db_paths.nhlng.is_none(), "NHL18 does not match an NHL17 param.sfo");
    }

    /// Live check against the operator's real shadPS4 install. Skipped when the
    /// drive isn't mounted. Every in-scope title present must at least resolve its
    /// game folder (install found), even if the big files aren't extracted yet.
    #[test]
    fn ps4_real_shadps4_detects_all_installed_titles() {
        let root = Path::new(r"H:\Emulators\ShadPS4");
        if !root.join("Games").is_dir() {
            eprintln!("skipping: {} not present", root.display());
            return;
        }
        // (game_id, present-on-this-machine). NHL19 isn't installed here.
        for gid in ["nhl15", "nhl16", "nhl17", "nhl18", "nhl20", "nhl21"] {
            let r = detect_install(gid, Platform::Ps4, root, None);
            // Either the loose db resolved, or the game dir was found and flagged
            // for extraction — both mean the install was detected.
            let detected = r.db_paths.nhlng.is_some() || r.extract_needed;
            assert!(detected, "{gid}: install not detected (regression)");
            eprintln!(
                "{gid}: nhlng={} extract_needed={}",
                r.db_paths.nhlng.is_some(),
                r.extract_needed
            );
        }
    }
}
