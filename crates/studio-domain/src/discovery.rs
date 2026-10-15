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
/// NHL 07 is Xbox 360 only in this tool. NHL 08–09 have verified PS3 anchors; their product codes
/// below are the regional ids used for install/save discovery. The Xbox title ids are read from each
/// title's `default.xex`.
pub fn title_ids(game_id: &str) -> Option<TitleIds> {
    let ids = match game_id {
        "nhl07" => ("", "", "454107F3"),
        "nhl08" => ("BLUS30062", "", "45410820"),
        "nhl09" => ("BLUS30168", "", "45410872"),
        "nhl10" => ("BLUS30345", "BLES00567", "454108BA"),
        "nhl11" => ("BLUS30568", "BLES00984", "454108FD"),
        "nhl12" => ("BLUS30791", "BLES01357", "45410964"),
        "nhl13" => ("BLUS31003", "BLES01691", "4541099B"),
        "nhl14" => ("BLUS31179", "BLES01853", "454109BE"),
        "nhl15" => ("BLUS31436", "BLES02037", "454109DC"),
        "nhllegacy" => ("BLUS31540", "BLES02153", "454109EC"),
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
    /// PS4: the shadPS4 user profile folder the save belongs to (e.g. `1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// PS4: the title id directory the save belongs to (e.g. `CUSA18182`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_id: Option<String>,
    /// Why the save can't be opened (unreadable, or a format the codec doesn't recognise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// One PS4 game folder discovery considered, identified from its own `sce_sys/param.sfo`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GameCandidate {
    pub path: String,
    /// `param.sfo` TITLE, e.g. `NHL™ 21`.
    pub title: Option<String>,
    /// `param.sfo` TITLE_ID, e.g. `CUSA18182`.
    pub title_id: Option<String>,
    /// `param.sfo` CONTENT_ID; its prefix names the region (`UP` / `EP` …).
    pub content_id: Option<String>,
    /// `param.sfo` APP_VER (the installed patch level).
    pub app_version: Option<String>,
    /// Whether this folder is the requested title.
    pub matches: bool,
    /// Why it isn't the requested title, or why it can't be identified.
    pub reason: Option<String>,
}

/// A discovery finding shown to the user: a stable kind and a readable explanation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveryProblem {
    /// `game_location_unreadable` | `game_not_found` | `game_wrong_title` | `game_unidentified` |
    /// `game_ambiguous` | `game_location_incomplete` | `database_missing` | `metadata_missing` |
    /// `save_location_unreadable` | `save_inventory_incomplete` | `saves_not_found` |
    /// `saves_other_title`. An `…_incomplete` kind replaces "not found" when part of the location
    /// couldn't be listed.
    pub kind: String,
    pub message: String,
}

impl DiscoveryProblem {
    fn new(kind: &str, message: String) -> Self {
        DiscoveryProblem { kind: kind.to_string(), message }
    }
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
    /// PS4: the matched game folder (database, archives and art resolve from it).
    #[serde(default)]
    pub game_dir: Option<String>,
    /// PS4: the save-data location searched (the explicit one, or the older layout under the root).
    #[serde(default)]
    pub save_root: Option<String>,
    /// PS4: every game folder considered, matching or not.
    #[serde(default)]
    pub game_candidates: Vec<GameCandidate>,
    /// Why something wasn't resolved (missing game, wrong title, unreadable location, no saves …).
    #[serde(default)]
    pub problems: Vec<DiscoveryProblem>,
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
/// profile save location, including relocated RPCS3 saves. For Xbox, a
/// `None` `save_path` falls back to scanning the default Xenia content directory; for PS4 it falls
/// back to the older layout under `root` (see [`detect_ps4`]).
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
        game_dir: None,
        save_root: None,
        game_candidates: Vec::new(),
        problems: Vec::new(),
    };

    match platform {
        Platform::Ps3 => {
            if let Some(ids) = title_ids(game_id) {
                detect_ps3(root, &ids, &mut report);
                if let Some(path) = save_path {
                    // An explicit save location also supports relocated RPCS3 user data.
                    report.saves = scan_saves(path);
                }
            }
        }
        Platform::Xbox => {
            if let Some(ids) = title_ids(game_id) {
                detect_xbox(root, &ids, save_path, &mut report);
            }
        }
        Platform::Ps4 => detect_ps4(root, save_path, game_id, ps4_title_ids(game_id), &mut report),
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
    let missing_db = report.db_paths.nhlng.is_none() || report.db_paths.localization.is_none();
    if missing_db && !crate::extract::find_big_archives(install_dir).is_empty() {
        report.extract_needed = true;
    }
}

fn detect_ps3(root: &Path, ids: &TitleIds, report: &mut InstallReport) {
    // Xbox-only titles (currently NHL 07) carry empty PS3 codes; nothing to detect and an empty prefix
    // would match every save folder in the scan below, so bail early.
    if ids.ps3_us.is_empty() && ids.ps3_eu.is_empty() {
        return;
    }
    // Accept a disc root, PS3_GAME, USRDIR, or a relocated virtual HDD directly.
    // Do not recursively guess among unrelated games in a user's library.
    let hdd = if root.join("dev_hdd0").is_dir() { root.join("dev_hdd0") } else { root.to_path_buf() };
    let game_dir = hdd.join("game");
    let mut candidates = vec![root.join("PS3_GAME/USRDIR"), root.join("USRDIR")];
    candidates.extend([ids.ps3_us, ids.ps3_eu].iter().filter(|tid| !tid.is_empty()).map(|tid| game_dir.join(tid).join("USRDIR")));
    // A recognised disc layout needs no second recursive search from its parent.
    if !candidates.iter().any(|p| p.is_dir())
        && (root.join("EBOOT.BIN").is_file() || resolve_db_paths(root).nhlng.is_some()) {
        candidates.push(root.to_path_buf());
    }
    let install_dir = candidates.into_iter().find(|p| p.is_dir());
    if let Some(dir) = &install_dir {
        report.install_dir = Some(dir.to_string_lossy().to_string());
        report.db_paths = resolve_db_paths(dir);
        note_extract_needed(dir, report);
    }

    // Saves: <root>/dev_hdd0/home/<USER>/savedata/<TITLEID…>/SYS-DATA. RPCS3 can have multiple
    // users and the active user is not necessarily 00000001, so inspect every user directory.
    let home = hdd.join("home");
    if let Ok(users) = std::fs::read_dir(&home) {
        for user in users.flatten().filter(|e| e.path().is_dir()) {
            let savedata = user.path().join("savedata");
            if let Ok(entries) = std::fs::read_dir(&savedata) {
                for entry in entries.flatten() {
                    if !entry.path().is_dir() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_uppercase();
                    let matches_region = (!ids.ps3_us.is_empty() && name.starts_with(ids.ps3_us))
                        || (!ids.ps3_eu.is_empty() && name.starts_with(ids.ps3_eu));
                    if !matches_region {
                        continue;
                    }
                    if let Some(slot) = classify_save(&entry.path().join("SYS-DATA")) {
                        report.saves.push(slot);
                    }
                }
            }
        }
    }
}

fn detect_xbox(root: &Path, ids: &TitleIds, save_path: Option<&Path>, report: &mut InstallReport) {
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

/// PS4 discovery (N21-001). The **game folder** and the **save-data folder** are independent
/// locations: shadPS4 keeps games wherever the user installed them and saves in its own user data
/// (`%APPDATA%\shadPS4\savedata` on Windows), neither of which need sit beside `shadPS4.exe`.
///
/// * `root` is the game location: a game folder picked directly (any name — it is identified by its
///   own `sce_sys/param.sfo`, never by the folder name), or a parent library holding game folders
///   (`<root>/Games/<game>` or `<root>/<game>`, the older emulator-folder layout). Every game folder
///   considered is reported in `game_candidates`; exactly one must match the title, otherwise nothing
///   is resolved and the reason is reported (none, wrong title, unidentified, several).
/// * `save_path` is the save-data location: the savedata root (`<root>/<profile>/<TITLE_ID>/…`), one
///   profile (`<profile>/<TITLE_ID>/…`) or one title directory (`<TITLE_ID>/<save>/DATA`). Every
///   matching save of every profile and applicable title id is listed with its profile and title id.
///   When it is `None` the older layout is searched under `root` (`user/savedata`, `savedata`, or
///   `root` as a profile folder), so a remembered emulator-folder configuration still resolves.
///
/// The database, metadata and dictionary resolve from the matched game folder only. Discovery
/// reads directory listings, `param.sfo` and save headers; it never writes.
fn detect_ps4(
    root: &Path,
    save_path: Option<&Path>,
    game_id: &str,
    known: &[&str],
    report: &mut InstallReport,
) {
    let label = ps4_title_label(game_id);
    let year = ps4_title_year(game_id);

    // --- Game folder -----------------------------------------------------------------------
    let mut matched: Vec<PathBuf> = Vec::new();
    match std::fs::read_dir(root) {
        Err(e) => report.problems.push(DiscoveryProblem::new(
            "game_location_unreadable",
            format!("The game location {} can't be read ({e}).", root.display()),
        )),
        Ok(_) => {
            let direct = is_ps4_game_dir(root);
            let mut unread = Unreadable::new();
            let dirs = if direct { vec![root.to_path_buf()] } else { ps4_library_children(root, &mut unread) };
            for dir in dirs {
                let cand = ps4_identify(&dir, year);
                if cand.matches {
                    matched.push(dir);
                }
                report.game_candidates.push(cand);
            }
            match matched.len() {
                1 => {}
                0 if direct => {
                    let c = &report.game_candidates[0];
                    let (kind, why) = if c.title.is_some() { ("game_wrong_title", "is a different game") } else { ("game_unidentified", "can't be identified") };
                    report.problems.push(DiscoveryProblem::new(
                        kind,
                        format!("The selected folder {why}: {}. Choose the {label} game folder.", c.reason.clone().unwrap_or_default()),
                    ));
                }
                // An incomplete library scan never claims the game is absent (see below).
                0 if !unread.is_empty() => {}
                0 => {
                    let others: Vec<String> = report
                        .game_candidates
                        .iter()
                        .map(|c| match (&c.title, &c.title_id) {
                            (Some(t), Some(id)) => format!("{t} ({id})"),
                            (Some(t), None) => t.clone(),
                            _ => format!("{} (unidentified)", file_name(&c.path)),
                        })
                        .collect();
                    let found = if others.is_empty() { String::new() } else { format!(" Found: {}.", others.join(", ")) };
                    report.problems.push(DiscoveryProblem::new(
                        "game_not_found",
                        format!("No {label} game folder was found in {}.{found} Choose the {label} game folder itself.", root.display()),
                    ));
                }
                n => report.problems.push(DiscoveryProblem::new(
                    "game_ambiguous",
                    format!("{n} {label} game folders were found in {}. Choose the one to open.", root.display()),
                )),
            }
            if !unread.is_empty() {
                let lead = if matched.is_empty() {
                    format!("No {label} game folder was found among the folders of {} that could be read, but", root.display())
                } else {
                    format!("In {},", root.display())
                };
                report.problems.push(DiscoveryProblem::new(
                    "game_location_incomplete",
                    format!("{lead} {} couldn't be read, so game folders there weren't checked: {}.", folders(unread.len()), describe(&unread)),
                ));
            }
        }
    }
    if let [game_dir] = matched.as_slice() {
        report.game_dir = Some(path_string(game_dir.clone()));
        resolve_ps4_game_files(game_dir, report);
        let c = report.game_candidates.iter().find(|c| Path::new(&c.path) == game_dir.as_path()).cloned().unwrap_or_default();
        let id = c.title_id.as_deref().map(|i| format!(" ({i})")).unwrap_or_default();
        if report.db_paths.nhlng.is_none() && !report.extract_needed {
            report.problems.push(DiscoveryProblem::new(
                "database_missing",
                format!("{label}{id} was found at {}, but its nhl_game.db wasn't found there or in its archives.", game_dir.display()),
            ));
        } else if report.db_paths.nhlng.is_some() && report.db_paths.meta_xml.is_none() {
            report.problems.push(DiscoveryProblem::new(
                "metadata_missing",
                format!("{label}{id}: nhl_game.db was found, but nhl_game-meta.xml is missing beside it."),
            ));
        }
    }

    // --- Saves -----------------------------------------------------------------------------
    // Applicable title ids: the title's known regional ids plus every matched install's own id.
    let mut ids: Vec<String> = known.iter().map(|s| s.to_ascii_uppercase()).collect();
    for c in report.game_candidates.iter().filter(|c| c.matches) {
        if let Some(id) = &c.title_id {
            if !ids.iter().any(|k| k.eq_ignore_ascii_case(id)) {
                ids.push(id.to_ascii_uppercase());
            }
        }
    }
    // Folders of the save-data location whose listing failed: recorded, never read as "no saves".
    let mut unread = Unreadable::new();
    let title_dirs = match save_path {
        Some(sp) => {
            report.save_root = Some(path_string(sp.to_path_buf()));
            if let Err(e) = std::fs::read_dir(sp) {
                report.problems.push(DiscoveryProblem::new(
                    "save_location_unreadable",
                    format!("The save-data location {} can't be read ({e}).", sp.display()),
                ));
                return;
            }
            ps4_title_dirs(sp, &mut unread)
        }
        None => {
            // The older emulator-folder layout: saves under the picked root itself.
            let legacy = child_ci(root, "user")
                .and_then(|u| child_ci(&u, "savedata"))
                .or_else(|| child_ci(root, "savedata"));
            match legacy {
                Some(sd) => {
                    report.save_root = Some(path_string(sd.clone()));
                    ps4_title_dirs(&sd, &mut unread)
                }
                None => {
                    // The game location itself isn't a declared save location: its unreadable folders
                    // are the game scan's concern (reported above), not the save inventory's.
                    let found = ps4_title_dirs(root, &mut Unreadable::new());
                    if !found.is_empty() {
                        report.save_root = Some(path_string(root.to_path_buf()));
                    }
                    found
                }
            }
        }
    };
    let mut other_titles: Vec<String> = Vec::new();
    for (profile, dir) in &title_dirs {
        let title_id = file_name(&path_string(dir.clone())).to_ascii_uppercase();
        if !ids.iter().any(|k| *k == title_id) {
            if !other_titles.contains(&title_id) {
                other_titles.push(title_id);
            }
            continue;
        }
        for save in ps4_save_dirs(dir, &mut unread) {
            let data = save.join("DATA");
            let name = file_name(&path_string(save.clone()));
            // A save is "writable" only if the codec recognises its inner LZ4 roster TDB; reading
            // a DATA file is never taken as support.
            let (writable, problem) = match std::fs::read(&data) {
                Ok(b) if tdb_savedata::is_ps4_save(&b) => (true, None),
                Ok(_) => (false, Some("Studio can't read this save's format; it is listed only.".to_string())),
                Err(e) => (false, Some(format!("This save couldn't be read ({e}).")) ),
            };
            report.saves.push(SaveSlot {
                kind: ps4_save_kind(&name).to_string(),
                path: path_string(data),
                container_version: "ps4".to_string(),
                writable,
                profile: profile.clone(),
                title_id: Some(title_id.clone()),
                problem,
            });
        }
    }
    // An unreadable folder named for another title can't hold this title's saves; every other one
    // (profiles, this title's id, save folders) may, so the inventory is reported incomplete.
    unread.retain(|p, _| {
        let name = file_name(&path_string(p.clone())).to_ascii_uppercase();
        let other_title = name.len() == 9 && name.starts_with("CUSA") && name[4..].bytes().all(|b| b.is_ascii_digit()) && !ids.contains(&name);
        !other_title
    });
    if !unread.is_empty() {
        let lead = if report.saves.is_empty() {
            format!("No {label} saves were found in the folders that could be read, but")
        } else {
            "This save list may be incomplete:".to_string()
        };
        report.problems.push(DiscoveryProblem::new(
            "save_inventory_incomplete",
            format!("{lead} {} couldn't be read, so saves there weren't checked: {}.", folders(unread.len()), describe(&unread)),
        ));
    } else if report.saves.is_empty() {
        let message = match (save_path, other_titles.is_empty()) {
            (_, false) => format!(
                "No {label} saves were found{}; the saves there belong to {} (expected {}).",
                report.save_root.as_deref().map(|r| format!(" in {r}")).unwrap_or_default(),
                other_titles.join(", "),
                ids.join(" or "),
            ),
            (Some(sp), true) => format!("No {label} saves were found in {}.", sp.display()),
            (None, true) => format!("No save-data folder was chosen and none was found in {}; choose the shadPS4 savedata folder.", root.display()),
        };
        let kind = if other_titles.is_empty() { "saves_not_found" } else { "saves_other_title" };
        report.problems.push(DiscoveryProblem::new(kind, message));
    }
}

/// The databases of one matched PS4 game folder: the standard t3db / loc locations, then a bounded
/// recursive search (an extracted tree such as `databigs/Extracted/nhl/...`), plus the extraction
/// flags. Unchanged from the emulator-root discovery except that the folder is now given directly.
fn resolve_ps4_game_files(game_dir: &Path, report: &mut InstallReport) {
    let t3db = game_dir.join("nhl/data/common/gamemodes/database/t3db");
    let nhlng = some_file(t3db.join("nhl_game.db")).or_else(|| find_file(game_dir, "nhl_game.db", 9));
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
        .or_else(|| find_file(game_dir, "eng_us.db", 9))
        .map(path_string);
    note_extract_needed(game_dir, report);
    // PS4 ships its databases *loose* but keeps the art (meshes / textures / portraits)
    // packed inside the databigs, so a resolved db is **not** proof the install is fully
    // unpacked — unlike Xbox, where the db and art come out of the same `.big` set. Flag
    // extraction whenever the kettle art tree is still missing but `.big` archives are
    // present, so asset editing has files to work with. (Extraction leaves the `.big`
    // files in place, so `complete()` — not their absence — is the "already unpacked" signal.)
    if !report.extract_needed
        && !crate::assets::AssetRoots::resolve(game_dir, Platform::Ps4).complete()
        && !crate::extract::find_big_archives(game_dir).is_empty()
    {
        report.extract_needed = true;
    }
}

/// `true` if `dir` looks like a PS4 game folder itself (rather than a library holding them): it has
/// the package metadata `sce_sys/param.sfo`, or the game's `nhl` / `databigs` trees.
fn is_ps4_game_dir(dir: &Path) -> bool {
    child_ci(dir, "sce_sys").is_some_and(|s| s.join("param.sfo").is_file())
        || child_ci(dir, "nhl").is_some()
        || child_ci(dir, "databigs").is_some()
}

/// The game folders of a library: the children of `<root>/Games` and of `<root>` that look like PS4
/// game folders (one level only — no deeper scan), sorted and de-duplicated. A base or child folder
/// whose listing fails is recorded in `unread` rather than skipped as if it held no game.
fn ps4_library_children(root: &Path, unread: &mut Unreadable) -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Some(games) = child_ci(root, "Games") {
        bases.push(games);
    }
    bases.push(root.to_path_buf());
    let mut out: Vec<PathBuf> = Vec::new();
    for base in bases {
        for k in sorted_dirs(&base, unread) {
            if let Err(e) = std::fs::read_dir(&k) {
                unread.insert(k, e.to_string());
                continue;
            }
            if is_ps4_game_dir(&k) && !out.contains(&k) {
                out.push(k);
            }
        }
    }
    out
}

/// Identify one game folder from its own `sce_sys/param.sfo`: an NHL title whose TITLE year is the
/// requested one and whose CONTENT_ID (when present) names the same year. The folder name is never
/// used. `year` is `None` only for an unknown game id, which matches nothing.
fn ps4_identify(dir: &Path, year: Option<u32>) -> GameCandidate {
    let mut c = GameCandidate { path: path_string(dir.to_path_buf()), ..GameCandidate::default() };
    let Some(sfo) = child_ci(dir, "sce_sys").map(|s| s.join("param.sfo")).filter(|p| p.is_file()) else {
        c.reason = Some(format!("{} has no sce_sys/param.sfo, so the game can't be confirmed", dir.display()));
        return c;
    };
    let data = match std::fs::read(&sfo) {
        Ok(d) => d,
        Err(e) => {
            c.reason = Some(format!("{} couldn't be read ({e})", sfo.display()));
            return c;
        }
    };
    c.title = sfo_string_value(&data, "TITLE");
    c.title_id = sfo_string_value(&data, "TITLE_ID");
    c.content_id = sfo_string_value(&data, "CONTENT_ID");
    c.app_version = sfo_string_value(&data, "APP_VER");
    let Some(title) = c.title.clone() else {
        c.reason = Some(format!("{} has no TITLE", sfo.display()));
        return c;
    };
    let is_nhl = title.trim_start().to_ascii_uppercase().starts_with("NHL");
    let title_year = sfo_title_year(&title);
    let content_year_ok = c.content_id.as_deref().is_none_or(|cid| {
        let label = cid.rsplit('-').next().unwrap_or(cid);
        year.is_some_and(|y| label.ends_with(&format!("20{y:02}")))
    });
    c.matches = is_nhl && year.is_some() && title_year == year && content_year_ok;
    if !c.matches {
        let id = c.title_id.as_deref().map(|i| format!(" ({i})")).unwrap_or_default();
        c.reason = Some(if !is_nhl || title_year != year {
            format!("{title}{id}")
        } else {
            format!("{title}{id}, whose content id {} names a different year", c.content_id.clone().unwrap_or_default())
        });
    }
    c
}

/// The title directories reachable from a save-data selection, with their profile name. Accepts the
/// title directory itself (`<TITLE_ID>/<save>/DATA`), one profile (`<profile>/<TITLE_ID>/…`) or the
/// savedata root (`<root>/<profile>/<TITLE_ID>/…`) — the first level that yields any title directory
/// wins, so a selection is read at exactly one level. Bounded to two levels below the selection.
/// Every folder on the way whose listing fails is recorded in `unread`.
fn ps4_title_dirs(sel: &Path, unread: &mut Unreadable) -> Vec<(Option<String>, PathBuf)> {
    let name_of = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    if has_save_with_data(sel, unread) {
        return vec![(sel.parent().and_then(name_of), sel.to_path_buf())];
    }
    let children = sorted_dirs(sel, unread);
    let mut profile_level = Vec::new();
    for c in &children {
        if has_save_with_data(c, unread) {
            profile_level.push((name_of(sel), c.clone()));
        }
    }
    if !profile_level.is_empty() {
        return profile_level;
    }
    let mut root_level = Vec::new();
    for profile in &children {
        for t in sorted_dirs(profile, unread) {
            if has_save_with_data(&t, unread) {
                root_level.push((name_of(profile), t));
            }
        }
    }
    root_level
}

/// The save folders (those holding a `DATA` file) of one title directory, sorted.
fn ps4_save_dirs(title_dir: &Path, unread: &mut Unreadable) -> Vec<PathBuf> {
    sorted_dirs(title_dir, unread).into_iter().filter(|d| d.join("DATA").is_file()).collect()
}

/// Folders whose listing failed during a bounded scan, with the error: reported to the user as an
/// incomplete scan, never treated as empty.
type Unreadable = std::collections::BTreeMap<PathBuf, String>;

/// The direct child directories of `dir`, sorted; a listing failure is recorded in `unread` (and
/// yields no children).
fn sorted_dirs(dir: &Path, unread: &mut Unreadable) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(it) => it
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect(),
        Err(e) => {
            unread.insert(dir.to_path_buf(), e.to_string());
            Vec::new()
        }
    };
    v.sort();
    v
}

/// "1 folder" / "3 folders".
fn folders(n: usize) -> String {
    if n == 1 { "1 folder".to_string() } else { format!("{n} folders") }
}

/// The unreadable folders as `path (error)`, joined for a message.
fn describe(unread: &Unreadable) -> String {
    unread.iter().map(|(p, e)| format!("{} ({e})", p.display())).collect::<Vec<_>>().join("; ")
}

/// The NHL year in a `param.sfo` TITLE (`"NHL® 15"` → 15): the last run of digits. The `®` / `™`
/// is mojibake in some regions, so only the digits are trusted.
fn sfo_title_year(title: &str) -> Option<u32> {
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
/// blob). Minimal reader: enough to pull `TITLE`/`TITLE_ID`/`CONTENT_ID`/`APP_VER`. Returns `None`
/// on any malformation rather than panicking on untrusted input.
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
        let key_off = data
            .get(e..e + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)?;
        let len = u32_at(e + 4)?;
        let data_off = u32_at(e + 0x0C)?;
        // Key: NUL-terminated at key_table + key_off.
        let kstart = key_table.checked_add(key_off)?;
        let kend = data
            .get(kstart..)?
            .iter()
            .position(|&b| b == 0)
            .map(|p| kstart + p)?;
        let k = std::str::from_utf8(&data[kstart..kend]).ok()?;
        if k != key {
            continue;
        }
        let vstart = data_table.checked_add(data_off)?;
        let vslice = data.get(vstart..vstart.checked_add(len)?)?;
        let vend = vslice.iter().position(|&b| b == 0).unwrap_or(vslice.len());
        return Some(String::from_utf8_lossy(&vslice[..vend]).into_owned());
    }
    None
}

/// `true` if `cusa_dir` has at least one subfolder containing a `DATA` file (a PS4 save).
/// A listing failure is recorded in `unread` (and answers `false`).
fn has_save_with_data(cusa_dir: &Path, unread: &mut Unreadable) -> bool {
    match std::fs::read_dir(cusa_dir) {
        Ok(it) => it.flatten().any(|e| e.path().is_dir() && e.path().join("DATA").is_file()),
        Err(e) => {
            unread.insert(cusa_dir.to_path_buf(), e.to_string());
            false
        }
    }
}

/// The user-facing title name for a PS4 game id (`nhl21` → `NHL 21`).
fn ps4_title_label(game_id: &str) -> String {
    ps4_title_year(game_id).map(|y| format!("NHL {y:02}")).unwrap_or_else(|| game_id.to_string())
}

/// The final path component of a path string.
fn file_name(p: &str) -> String {
    Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.to_string())
}

/// The suggested PS4 save-data location for the current Windows user: shadPS4's default user data
/// `%APPDATA%\shadPS4\savedata`, from the environment (like [`default_xenia_content`]); `None` when
/// the environment has no roaming application-data folder. A suggestion only — never scanned unless
/// the user accepts it as the save location.
pub fn ps4_default_save_root() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join("shadPS4").join("savedata"))
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
    let names = ["nhlng.db", "NHL07NG.db", "nhl_eng_us.db", "nhlng-meta.xml", "NHL07NG-meta.xml"];
    let mut found: Vec<Option<PathBuf>> = vec![None; names.len()];
    // Check the shipped layouts first; extracted art can contain hundreds of
    // thousands of files and is not needed to locate these three inputs.
    for relative in ["", "db", "data/db", "fe/loc"] {
        for (i, name) in names.iter().enumerate() {
            let path = dir.join(relative).join(name);
            if found[i].is_none() && path.is_file() { found[i] = Some(path); }
        }
    }
    let complete = |f: &[Option<PathBuf>]| (f[0].is_some() || f[1].is_some()) && f[2].is_some() && (f[3].is_some() || f[4].is_some());
    if !complete(&found) {
        // One fallback traversal for all missing inputs, rather than a full
        // directory walk per filename (including the absent NHL07 aliases).
        let mut stack = vec![(dir.to_path_buf(), 0usize)];
        while let Some((parent, depth)) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(parent) else { continue };
            for entry in entries.flatten() {
                let Ok(ft) = entry.file_type() else { continue };
                if ft.is_dir() {
                    if depth < 6 { stack.push((entry.path(), depth + 1)); }
                } else if let Some(i) = names.iter().position(|n| entry.file_name().to_string_lossy().eq_ignore_ascii_case(n)) {
                    if found[i].is_none() { found[i] = Some(entry.path()); }
                }
            }
        }
    }
    DbPaths {
        nhlng: found[0].take().or_else(|| found[1].take()).map(path_string),
        localization: found[2].take().map(path_string),
        meta_xml: found[3].take().or_else(|| found[4].take()).map(path_string),
    }
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
            } else if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
            {
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
        profile: None,
        title_id: None,
        problem: None,
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
        ContainerVersion::RawV3 => "v3",
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

    #[test]
    fn database_discovery_supports_standard_and_relocated_layouts() {
        for relative in ["db", "unusual/nested"] {
            let tmp = TempTree::new(&format!("db-discovery-{}", relative.replace('/', "-")));
            let db = tmp.0.join(relative);
            fs::create_dir_all(&db).unwrap();
            for name in ["NHL07NG.db", "NHL07NG-meta.xml", "nhl_eng_us.db"] {
                fs::write(db.join(name), b"fixture").unwrap();
            }
            let paths = resolve_db_paths(&tmp.0);
            assert_eq!(paths.nhlng.map(PathBuf::from), Some(db.join("NHL07NG.db")));
            assert_eq!(paths.meta_xml.map(PathBuf::from), Some(db.join("NHL07NG-meta.xml")));
            assert_eq!(paths.localization.map(PathBuf::from), Some(db.join("nhl_eng_us.db")));
        }
    }

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
        fs::copy(&from, &to)
            .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", from.display(), to.display()));
    }

    #[test]
    fn ps3_disc_and_relocated_hdd_roots_resolve_without_emulator_layout() {
        let t = TempTree::new("ps3-disc-roots");
        let a = anchors().join("ps3/nhl08/databases");
        let disc = t.0.join("arbitrary disc name");
        let game = disc.join("PS3_GAME/USRDIR");
        copy(a.join("nhlng.db"), game.join("db/nhlng.db"));
        write_big(&game.join("data0.big"), &[("fe/loc/nhl_eng_us.db", b"dictionary")]);
        for root in [&disc, &disc.join("PS3_GAME"), &game] {
            let report = detect_install("nhl08", Platform::Ps3, root, None);
            assert_eq!(Path::new(report.install_dir.as_deref().unwrap()), game.as_path());
            assert!(report.db_paths.nhlng.is_some());
            assert!(report.extract_needed);
        }
        let hdd = t.0.join("moved virtual disk");
        let installed = hdd.join("game/BLUS30062/USRDIR");
        copy(a.join("nhlng.db"), installed.join("db/nhlng.db"));
        let report = detect_install("nhl08", Platform::Ps3, &hdd, None);
        assert_eq!(Path::new(report.install_dir.as_deref().unwrap()), installed.as_path());
        assert!(report.db_paths.nhlng.is_some());
    }

    #[test]
    fn ps3_detect_resolves_modding_minimum() {
        let a = anchors().join("ps3/nhl14");
        let t = TempTree::new("ps3");
        let root = &t.0;
        let usrdir = root.join("dev_hdd0/game/BLUS31179/USRDIR");
        copy(a.join("databases/nhlng.db"), usrdir.join("nhlng.db"));
        copy(
            a.join("databases/nhlng-meta.xml"),
            usrdir.join("nhlng-meta.xml"),
        );
        copy(
            a.join("databases/nhl_eng_us.db"),
            usrdir.join("nhl_eng_us.db"),
        );
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

        let moved_saves = root.join("separate save drive");
        copy(a.join("savefiles/BLUS311790200 - ROSTER/SYS-DATA"),
            moved_saves.join("BLUS311790200 - ROSTER/SYS-DATA"));
        let explicit = detect_install("nhl14", Platform::Ps3, root, Some(&moved_saves));
        assert_eq!(explicit.saves.len(), 1, "explicit location replaces automatic save discovery");
        assert!(Path::new(&explicit.saves[0].path).starts_with(&moved_saves));
    }

    #[test]
    fn ps3_legacy_detects_correct_title_id_and_non_default_user() {
        let a = anchors().join("ps3/nhllegacy");
        let t = TempTree::new("ps3-legacy-user");
        let root = &t.0;
        let usrdir = root.join("dev_hdd0/game/BLUS31540/USRDIR");
        copy(
            a.join("databases/nhlng.db"),
            usrdir.join("data/db/nhlng.db"),
        );
        copy(
            a.join("databases/nhlng-meta.xml"),
            usrdir.join("data/db/nhlng-meta.xml"),
        );
        copy(
            a.join("databases/nhl_eng_us.db"),
            usrdir.join("data/db/nhl_eng_us.db"),
        );
        copy(
            a.join("savefiles/BLUS315400200 - ROSTER/SYS-DATA"),
            root.join("dev_hdd0/home/00000007/savedata/BLUS315400200 - ROSTER/SYS-DATA"),
        );

        let r = detect_install("nhllegacy", Platform::Ps3, root, None);
        assert!(
            r.minimum_met,
            "Legacy db and roster should be auto-detected"
        );
        assert!(r.db_paths.nhlng.is_some());
        assert!(r.db_paths.meta_xml.is_some());
        assert!(r.db_paths.localization.is_some());
        assert!(r.saves.iter().any(|s| s.kind == "roster"));
        assert!(r
            .install_dir
            .as_deref()
            .is_some_and(|p| p.contains("BLUS31540")));
    }

    #[test]
    fn xbox_detect_classifies_profile_as_settings() {
        let a = anchors().join("xbox360/nhl14");
        let t = TempTree::new("xbox");
        let install = t.0.join("install");
        copy(a.join("databases/nhlng.db"), install.join("nhlng.db"));
        copy(
            a.join("databases/nhlng-meta.xml"),
            install.join("nhlng-meta.xml"),
        );
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
        assert!(
            r.extract_needed,
            "packed .big + no loose db => extraction needed"
        );
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
        copy(
            a.join("databases/nhlng-meta.xml"),
            game.join("nhlng-meta.xml"),
        );

        let saves = t.0.join("saves");
        copy(
            a.join("savefiles/ROSTER 20260627054746/ROSTER 20260627054746"),
            saves.join("ROSTER/container"),
        );

        let r = detect_install("legacyrecomp", Platform::Recomp, &game, Some(&saves));
        assert!(r.db_paths.nhlng.is_some(), "recomp nhlng.db resolved");
        assert!(
            r.saves.iter().any(|s| s.kind == "roster"),
            "roster save scanned"
        );
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
        write_game_sfo(&root.join("Games").join(cusa), "NHL™ 16", cusa, "NHLICEHOCKEY2016");
        let t3db = root
            .join("Games")
            .join(cusa)
            .join("nhl/data/common/gamemodes/database/t3db");
        write_dummy(&t3db.join("nhl_game.db"));
        write_dummy(&t3db.join("nhl_game-meta.xml"));
        write_dummy(
            &root
                .join("Games")
                .join(cusa)
                .join("nhl/data/kettle/loc/eng_us.db"),
        );
        let roster = anchors().join("ps4/nhl16/savefiles/84f8102f - ROSTER/DATA");
        copy(
            roster,
            root.join("user/savedata/1")
                .join(cusa)
                .join("RosterCustom0/DATA"),
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
        let roster = r
            .saves
            .iter()
            .find(|s| s.kind == "roster")
            .expect("RosterCustom save");
        assert_eq!(roster.container_version, "ps4");
        assert!(
            roster.writable,
            "real NHL16 roster DATA is recognised by the PS4 codec"
        );
    }

    #[test]
    fn ps4_loose_db_but_packed_art_flags_extract_needed() {
        // PS4 ships the db loose, so the modding minimum is met immediately — but the art is
        // still inside a databig and no kettle rendering tree exists. Extraction must still be
        // offered (the Assets/Export pages have nothing to scan until it runs).
        let t = TempTree::new("ps4-packed-art");
        build_shadps4_tree(&t.0);
        let game = t.0.join("Games").join("CUSA01909");
        write_big(
            &game.join("databigs/inxp_00.big"),
            &[("nhl/data/kettle/x.gnf", b"packed")],
        );

        let r = detect_install("nhl16", Platform::Ps4, &t.0, None);
        assert!(
            r.db_paths.nhlng.is_some(),
            "loose nhl_game.db still resolves"
        );
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
        write_big(
            &game.join("databigs/inxp_00.big"),
            &[("nhl/data/kettle/x.gnf", b"packed")],
        );
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
        assert!(
            r.saves.iter().any(|s| s.kind == "roster"),
            "save still found"
        );
        assert!(
            r.db_paths.nhlng.is_none(),
            "no game db reachable from savedata alone"
        );
        assert!(
            !r.minimum_met,
            "without nhl_game.db + meta, PS4 minimum is unmet"
        );
    }

    /// Build a minimal Sony `param.sfo` blob carrying a single `TITLE` value.
    fn build_sfo(title: &str) -> Vec<u8> {
        build_sfo_kv(&[("TITLE", title)])
    }

    /// Build a minimal Sony `param.sfo` blob carrying the given utf-8 string keys.
    fn build_sfo_kv(pairs: &[(&str, &str)]) -> Vec<u8> {
        let mut keys = Vec::new();
        let mut vals = Vec::new();
        let mut index = Vec::new();
        for (k, v) in pairs {
            let mut val = v.as_bytes().to_vec();
            val.push(0);
            // index entry: key_offset, fmt(utf8=0x0204), len, max_len, data_offset
            index.extend_from_slice(&(keys.len() as u16).to_le_bytes());
            index.extend_from_slice(&0x0204u16.to_le_bytes());
            index.extend_from_slice(&(val.len() as u32).to_le_bytes());
            index.extend_from_slice(&(val.len() as u32).to_le_bytes());
            index.extend_from_slice(&(vals.len() as u32).to_le_bytes());
            keys.extend_from_slice(k.as_bytes());
            keys.push(0);
            vals.extend_from_slice(&val);
        }
        let key_table = 0x14 + index.len();
        let data_table = key_table + keys.len();
        let mut d = Vec::new();
        d.extend_from_slice(b"\x00PSF");
        d.extend_from_slice(&0x0101_0000u32.to_le_bytes()); // version
        d.extend_from_slice(&(key_table as u32).to_le_bytes());
        d.extend_from_slice(&(data_table as u32).to_le_bytes());
        d.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
        d.extend_from_slice(&index);
        d.extend_from_slice(&keys);
        d.extend_from_slice(&vals);
        d
    }

    /// Write a real-shaped `sce_sys/param.sfo` (TITLE, TITLE_ID, CONTENT_ID, APP_VER) into a game
    /// folder, as a shadPS4 install carries.
    fn write_game_sfo(game: &Path, title: &str, title_id: &str, label: &str) {
        let content_id = format!("EP0006-{title_id}_00-{label}");
        let sfo = build_sfo_kv(&[("TITLE", title), ("TITLE_ID", title_id), ("CONTENT_ID", &content_id), ("APP_VER", "01.00")]);
        fs::create_dir_all(game.join("sce_sys")).unwrap();
        fs::write(game.join("sce_sys/param.sfo"), sfo).unwrap();
    }

    #[test]
    fn sfo_year_extraction() {
        // Real installs: NHL 15/16/21 mojibake the ®, NHL 17/18/24 keep it.
        assert_eq!(ps4_title_year("nhl17"), Some(17));
        for (title, _year) in [
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
        assert!(
            r.minimum_met,
            "PS4 minimum met for the param.sfo-resolved install"
        );

        // A different title must NOT claim this install.
        let other = detect_install("nhl18", Platform::Ps4, &t.0, None);
        assert!(
            other.db_paths.nhlng.is_none(),
            "NHL18 does not match an NHL17 param.sfo"
        );
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

    // ---- N21-001: independent PS4 game and save-data locations ----------------------------

    /// A PS4 game folder at `game` (any name): real-shaped param.sfo + dummy loose databases.
    fn build_ps4_game(game: &Path, title: &str, title_id: &str, label: &str) {
        write_game_sfo(game, title, title_id, label);
        let t3db = game.join("nhl/data/common/gamemodes/database/t3db");
        write_dummy(&t3db.join("nhl_game.db"));
        write_dummy(&t3db.join("nhl_game-meta.xml"));
        write_dummy(&game.join("nhl/data/kettle/loc/eng_us.db"));
    }

    /// One shadPS4 save `<savedata>/<profile>/<title>/<save>/DATA`: the real NHL 16 roster
    /// container when `real`, otherwise bytes no codec recognises.
    fn put_save(savedata: &Path, profile: &str, title: &str, save: &str, real: bool) -> PathBuf {
        let data = savedata.join(profile).join(title).join(save).join("DATA");
        if real {
            copy(anchors().join("ps4/nhl16/savefiles/84f8102f - ROSTER/DATA"), data.clone());
        } else {
            write_dummy(&data);
        }
        data
    }

    fn kinds(r: &InstallReport) -> Vec<&str> {
        r.problems.iter().map(|p| p.kind.as_str()).collect()
    }

    #[test]
    fn ps4_direct_game_folder_with_unrelated_save_location() {
        let t = TempTree::new("n21-direct");
        // A game folder not named after its title id, far from any emulator folder.
        let game = t.0.join("drive-d/My Games/hockey twenty-one");
        build_ps4_game(&game, "NHL™ 21", "CUSA18182", "NHLICEHOCKEY2021");
        // Roaming-shaped saves somewhere else entirely.
        let savedata = t.0.join("Users/someone/AppData/Roaming/shadPS4/savedata");
        put_save(&savedata, "1", "CUSA18182", "RosterCustom0", true);
        put_save(&savedata, "1", "CUSA18182", "Profile0", false);

        let r = detect_install("nhl21", Platform::Ps4, &game, Some(&savedata));
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        assert_eq!(r.game_dir.as_deref(), Some(game.to_string_lossy().as_ref()));
        assert!(r.db_paths.nhlng.as_deref().is_some_and(|p| p.starts_with(game.to_string_lossy().as_ref())));
        assert!(r.db_paths.meta_xml.is_some() && r.db_paths.localization.is_some());
        assert!(r.minimum_met);
        assert_eq!(r.save_root.as_deref(), Some(savedata.to_string_lossy().as_ref()));
        assert_eq!(r.saves.len(), 2);
        let roster = r.saves.iter().find(|s| s.kind == "roster").unwrap();
        assert!(roster.writable && roster.problem.is_none());
        assert_eq!((roster.profile.as_deref(), roster.title_id.as_deref()), (Some("1"), Some("CUSA18182")));
        // A listed DATA file is not support: the profile bytes aren't a recognised container.
        let profile = r.saves.iter().find(|s| s.kind == "settings").unwrap();
        assert!(!profile.writable && profile.problem.is_some());
        let c = &r.game_candidates[0];
        assert!(c.matches);
        assert_eq!(c.title_id.as_deref(), Some("CUSA18182"));
        assert!(c.content_id.as_deref().is_some_and(|id| id.starts_with("EP")), "region from the content id");
    }

    #[test]
    fn ps4_game_identity_comes_from_param_sfo_not_the_folder_name() {
        let t = TempTree::new("n21-identity");
        // Named like NHL 21's title id, but the package metadata says NHL 20.
        let wrong = t.0.join("CUSA18182");
        build_ps4_game(&wrong, "NHL® 20", "CUSA15439", "NHLICEHOCKEY2020");
        let r = detect_install("nhl21", Platform::Ps4, &wrong, None);
        assert_eq!(kinds(&r)[0], "game_wrong_title");
        assert!(r.problems[0].message.contains("NHL® 20 (CUSA15439)"), "{}", r.problems[0].message);
        assert!(r.db_paths.nhlng.is_none() && r.game_dir.is_none() && !r.minimum_met, "a wrong title resolves nothing");
        // The same folder is accepted for the title it really is.
        assert!(detect_install("nhl20", Platform::Ps4, &wrong, None).minimum_met);

        // No param.sfo: a game tree whose title can't be confirmed is refused, whatever its name.
        let bare = t.0.join("CUSA18182-copy");
        write_dummy(&bare.join("nhl/data/common/gamemodes/database/t3db/nhl_game.db"));
        let r = detect_install("nhl21", Platform::Ps4, &bare, None);
        assert_eq!(kinds(&r)[0], "game_unidentified");
        assert!(r.db_paths.nhlng.is_none());

        // A content id naming another year is refused even under an NHL 21 TITLE.
        let mixed = t.0.join("mixed");
        build_ps4_game(&mixed, "NHL™ 21", "CUSA18182", "NHLICEHOCKEY2020");
        assert_eq!(kinds(&detect_install("nhl21", Platform::Ps4, &mixed, None))[0], "game_wrong_title");

        // Another series with the same number is not NHL 16 (the old year-only check took it).
        let fifa = t.0.join("lib/CUSA02138");
        build_ps4_game(&fifa, "FIFA 16", "CUSA02138", "FIFAFOOTBALL2016");
        let r = detect_install("nhl16", Platform::Ps4, &t.0.join("lib"), None);
        assert_eq!(kinds(&r)[0], "game_not_found");
        assert!(r.problems[0].message.contains("FIFA 16 (CUSA02138)"), "{}", r.problems[0].message);
    }

    #[test]
    fn ps4_library_layouts_still_resolve_and_several_matches_are_not_guessed() {
        let t = TempTree::new("n21-library");
        // The older shadPS4 folder: <root>/Games/<CUSA>, saves under <root>/user/savedata.
        build_shadps4_tree(&t.0);
        let r = detect_install("nhl16", Platform::Ps4, &t.0, None);
        assert!(r.minimum_met && r.problems.is_empty(), "{:?}", r.problems);
        assert!(r.saves.iter().any(|s| s.kind == "roster" && s.writable));
        assert!(r.save_root.as_deref().is_some_and(|s| s.ends_with("savedata")), "older layout saves found under the root");

        // A parent library holding arbitrarily named game folders.
        let lib = t.0.join("library");
        build_ps4_game(&lib.join("NHL 21 (EU)"), "NHL™ 21", "CUSA18182", "NHLICEHOCKEY2021");
        build_ps4_game(&lib.join("NHL 20"), "NHL® 20", "CUSA15439", "NHLICEHOCKEY2020");
        let r = detect_install("nhl21", Platform::Ps4, &lib, None);
        assert!(r.game_dir.as_deref().is_some_and(|g| g.ends_with("NHL 21 (EU)")), "{:?}", r.game_candidates);
        assert_eq!(r.game_candidates.len(), 2);
        assert!(r.db_paths.nhlng.is_some());

        // Two NHL 21 installs: both are reported and neither is chosen.
        build_ps4_game(&lib.join("NHL 21 second copy"), "NHL™ 21", "CUSA18182", "NHLICEHOCKEY2021");
        let r = detect_install("nhl21", Platform::Ps4, &lib, None);
        assert_eq!(kinds(&r), ["game_ambiguous", "saves_not_found"]);
        assert_eq!(r.game_candidates.iter().filter(|c| c.matches).count(), 2);
        assert!(r.db_paths.nhlng.is_none() && r.game_dir.is_none() && !r.minimum_met);
    }

    #[test]
    fn ps4_save_selection_at_root_profile_and_title_level() {
        let t = TempTree::new("n21-levels");
        let game = t.0.join("game");
        build_ps4_game(&game, "NHL™ 21", "CUSA18182", "NHLICEHOCKEY2021");
        let savedata = t.0.join("portable/user/savedata");
        put_save(&savedata, "1", "CUSA18182", "RosterCustom0", true);
        let paths = |sp: &Path| -> Vec<(PathBuf, Option<String>)> {
            let r = detect_install("nhl21", Platform::Ps4, &game, Some(sp));
            assert!(r.problems.is_empty(), "{}: {:?}", sp.display(), r.problems);
            r.saves.iter().map(|s| (PathBuf::from(&s.path), s.profile.clone())).collect()
        };
        let at_root = paths(&savedata);
        assert_eq!(at_root.len(), 1);
        assert_eq!(at_root[0].1.as_deref(), Some("1"));
        assert_eq!(paths(&savedata.join("1")), at_root, "profile folder");
        assert_eq!(paths(&savedata.join("1/CUSA18182")), at_root, "title folder");
        // A custom / portable location is just another explicit root.
        assert_eq!(paths(&t.0.join("portable/user/savedata")), at_root);
    }

    #[test]
    fn ps4_every_profile_and_title_variant_is_inventoried() {
        let t = TempTree::new("n21-profiles");
        let game = t.0.join("game20");
        build_ps4_game(&game, "NHL® 20", "CUSA15439", "NHLICEHOCKEY2020");
        let savedata = t.0.join("savedata");
        put_save(&savedata, "1", "CUSA15439", "RosterCustom0", true);
        put_save(&savedata, "1", "CUSA15439", "RosterCustom1", true);
        put_save(&savedata, "2", "CUSA15439", "RosterCustom0", true);
        // The title's other regional id (a known NHL 20 variant) is listed too, separately.
        put_save(&savedata, "2", "CUSA15447", "RosterCustom0", true);
        // Another game's saves are never listed as NHL 20's.
        put_save(&savedata, "1", "CUSA18182", "RosterCustom0", true);

        let r = detect_install("nhl20", Platform::Ps4, &game, Some(&savedata));
        let got: Vec<String> = r
            .saves
            .iter()
            .map(|s| {
                let save = file_name(Path::new(&s.path).parent().unwrap().to_str().unwrap());
                format!("{}/{}/{save}", s.profile.as_deref().unwrap(), s.title_id.as_deref().unwrap())
            })
            .collect();
        assert_eq!(
            got,
            [
                "1/CUSA15439/RosterCustom0",
                "1/CUSA15439/RosterCustom1",
                "2/CUSA15439/RosterCustom0",
                "2/CUSA15447/RosterCustom0",
            ]
        );
        assert!(r.saves.iter().all(|s| s.writable));

        // Choosing NHL 21's title folder for NHL 20 is refused, naming whose saves they are.
        let r = detect_install("nhl20", Platform::Ps4, &game, Some(&savedata.join("1/CUSA18182")));
        assert!(r.saves.is_empty());
        assert_eq!(kinds(&r), ["saves_other_title"]);
        assert!(r.problems[0].message.contains("CUSA18182"), "{}", r.problems[0].message);
    }

    #[test]
    fn ps4_missing_and_unreadable_locations_are_distinct() {
        let t = TempTree::new("n21-missing");
        let game = t.0.join("game");
        build_ps4_game(&game, "NHL™ 21", "CUSA18182", "NHLICEHOCKEY2021");

        let r = detect_install("nhl21", Platform::Ps4, &t.0.join("no-such-game"), Some(&t.0.join("no-such-saves")));
        assert_eq!(kinds(&r), ["game_location_unreadable", "save_location_unreadable"]);

        let empty = t.0.join("empty-saves");
        fs::create_dir_all(&empty).unwrap();
        let r = detect_install("nhl21", Platform::Ps4, &game, Some(&empty));
        assert_eq!(kinds(&r), ["saves_not_found"]);
        assert!(r.minimum_met, "the database still opens without a save (browse-only, as before)");

        // No save location chosen, and none beside a directly chosen game folder.
        let r = detect_install("nhl21", Platform::Ps4, &game, None);
        assert_eq!(kinds(&r), ["saves_not_found"]);
        assert!(r.problems[0].message.contains("No save-data folder was chosen"));

        // A matched game whose database is neither loose nor packed.
        let hollow = t.0.join("hollow");
        write_game_sfo(&hollow, "NHL™ 21", "CUSA18182", "NHLICEHOCKEY2021");
        let r = detect_install("nhl21", Platform::Ps4, &hollow, Some(&empty));
        assert_eq!(kinds(&r), ["database_missing", "saves_not_found"]);

        // An unreadable DATA (here: a directory where the file should be) is listed with its reason.
        let sd = t.0.join("odd-saves");
        fs::create_dir_all(sd.join("1/CUSA18182/RosterCustom0/DATA/inner")).unwrap();
        put_save(&sd, "1", "CUSA18182", "RosterCustom1", true);
        let r = detect_install("nhl21", Platform::Ps4, &game, Some(&sd));
        assert_eq!(r.saves.len(), 1, "a DATA that isn't a file isn't a save");
        assert!(r.saves[0].writable);

        // A DATA file that can't be read is listed with its reason and never offered for editing.
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let locked = put_save(&sd, "1", "CUSA18182", "RosterCustom2", true);
            let _hold = fs::OpenOptions::new().read(true).share_mode(0).open(&locked).unwrap();
            let r = detect_install("nhl21", Platform::Ps4, &game, Some(&sd));
            let slot = r.saves.iter().find(|s| s.path.contains("RosterCustom2")).unwrap();
            assert!(!slot.writable);
            assert!(slot.problem.as_deref().is_some_and(|p| p.contains("couldn't be read")), "{:?}", slot.problem);
        }
    }

    /// Hold `dir` open with no sharing so its listing fails (sharing violation) until the handle
    /// drops — an owned, self-restoring unreadable directory; asserts the precondition.
    #[cfg(windows)]
    fn lock_dir(dir: &Path) -> fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        let h = fs::OpenOptions::new().read(true).share_mode(0).custom_flags(FILE_FLAG_BACKUP_SEMANTICS).open(dir).unwrap();
        assert!(fs::read_dir(dir).is_err(), "{} must be unlistable for this case", dir.display());
        h
    }

    /// N21-001-R1: an unreadable profile or title folder is reported as an incomplete inventory —
    /// never dropped silently, and never turned into "no saves".
    #[cfg(windows)]
    #[test]
    fn ps4_unreadable_profile_and_title_folders_make_the_inventory_incomplete() {
        let t = TempTree::new("n21r1-unread");
        let game = t.0.join("game");
        build_ps4_game(&game, "NHL™ 21", "CUSA18182", "NHLICEHOCKEY2021");

        // An unreadable second profile beside a readable one: the readable save is listed, and the
        // list is marked incomplete naming the folder.
        let sd = t.0.join("savedata");
        put_save(&sd, "1", "CUSA18182", "RosterCustom0", true);
        put_save(&sd, "2", "CUSA18182", "RosterCustom0", true);
        {
            let _lock = lock_dir(&sd.join("2"));
            let r = detect_install("nhl21", Platform::Ps4, &game, Some(&sd));
            assert_eq!(r.saves.len(), 1);
            assert_eq!(r.saves[0].profile.as_deref(), Some("1"));
            assert_eq!(kinds(&r), ["save_inventory_incomplete"]);
            let m = &r.problems[0].message;
            assert!(m.starts_with("This save list may be incomplete: 1 folder couldn't be read"), "{m}");
            assert!(m.contains(&sd.join("2").display().to_string()), "{m}");
        }

        // The only title folder unreadable: no "no saves" claim — the scan says what it couldn't read.
        let sd2 = t.0.join("savedata2");
        put_save(&sd2, "1", "CUSA18182", "RosterCustom0", true);
        for sel in [sd2.clone(), sd2.join("1")] {
            let _lock = lock_dir(&sd2.join("1/CUSA18182"));
            let r = detect_install("nhl21", Platform::Ps4, &game, Some(&sel));
            assert!(r.saves.is_empty());
            assert_eq!(kinds(&r), ["save_inventory_incomplete"], "{}", sel.display());
            let m = &r.problems[0].message;
            assert!(m.starts_with("No NHL 21 saves were found in the folders that could be read, but 1 folder couldn't be read"), "{m}");
            assert!(m.contains("CUSA18182"), "{m}");
        }

        // A save folder is never listed — its DATA is opened by path — so an unlistable save folder
        // whose DATA is readable still lists normally (and an unreadable DATA is reported per slot,
        // see ps4_missing_and_unreadable_locations_are_distinct).
        {
            put_save(&sd2, "1", "CUSA18182", "RosterCustom1", true);
            let _lock = lock_dir(&sd2.join("1/CUSA18182/RosterCustom1"));
            let r = detect_install("nhl21", Platform::Ps4, &game, Some(&sd2));
            assert_eq!(r.saves.len(), 2);
            assert!(r.saves.iter().all(|s| s.writable && s.problem.is_none()));
            assert!(r.problems.is_empty(), "{:?}", r.problems);
        }

        // Another title's unreadable folder can't hold NHL 21 saves: not reported.
        let sd3 = t.0.join("savedata3");
        put_save(&sd3, "1", "CUSA18182", "RosterCustom0", true);
        put_save(&sd3, "1", "CUSA15439", "RosterCustom0", true);
        {
            let _lock = lock_dir(&sd3.join("1/CUSA15439"));
            let r = detect_install("nhl21", Platform::Ps4, &game, Some(&sd3));
            assert_eq!(r.saves.len(), 1);
            assert!(r.problems.is_empty(), "{:?}", r.problems);
        }
        // Everything readable again: the full inventory and no problem.
        let r = detect_install("nhl21", Platform::Ps4, &game, Some(&sd));
        assert_eq!(r.saves.len(), 2);
        assert!(r.problems.is_empty());
    }

    /// N21-001-R1: an unreadable folder in a game library is reported; with no match among the
    /// readable folders the game is not declared absent.
    #[cfg(windows)]
    #[test]
    fn ps4_unreadable_library_folders_are_reported_not_skipped() {
        let t = TempTree::new("n21r1-lib");
        let lib = t.0.join("library");
        build_ps4_game(&lib.join("NHL 21"), "NHL™ 21", "CUSA18182", "NHLICEHOCKEY2021");
        fs::create_dir_all(lib.join("locked")).unwrap();
        {
            let _lock = lock_dir(&lib.join("locked"));
            let r = detect_install("nhl21", Platform::Ps4, &lib, None);
            assert!(r.game_dir.as_deref().is_some_and(|g| g.ends_with("NHL 21")), "the readable match still resolves");
            assert_eq!(kinds(&r)[0], "game_location_incomplete");
            assert!(r.problems[0].message.contains("locked"), "{}", r.problems[0].message);
        }
        let only = t.0.join("only-locked");
        fs::create_dir_all(only.join("Games/CUSA18182")).unwrap();
        let _lock = lock_dir(&only.join("Games/CUSA18182"));
        let r = detect_install("nhl21", Platform::Ps4, &only, None);
        assert!(r.game_dir.is_none());
        assert!(!kinds(&r).contains(&"game_not_found"), "an incomplete scan doesn't claim the game is absent: {:?}", r.problems);
        assert_eq!(kinds(&r)[0], "game_location_incomplete");
        assert!(r.problems[0].message.starts_with("No NHL 21 game folder was found among the folders"), "{}", r.problems[0].message);
    }

    #[test]
    fn ps4_default_save_root_follows_the_user_environment() {
        let root = ps4_default_save_root();
        match std::env::var_os("APPDATA") {
            Some(a) => assert_eq!(root, Some(PathBuf::from(a).join("shadPS4").join("savedata"))),
            None => assert!(root.is_none()),
        }
    }
}
