//! NHL 24 PS4 (01.60, shadPS4) roster editing through the `game-mods` overlay (FB-W1 stage D).
//!
//! Opening extracts the database the game currently loads (Studio's overlay included) into a
//! working copy under `<install>\studio\working` and opens it with the ordinary editors. Saving
//! rebuilds Studio's overlay from the stock game around the edited database
//! ([`tdb_frostbite::nhl24`]) and only then writes the working copy, so the game is always the
//! source of truth: a working copy that failed to save is replaced on the next open.
//!
//! NHL 24 is not one of the 27 shipping targets. The session carries the game id `nhl24`,
//! which the PS4 player-row encoding contract and the player-style and nationality vocabularies
//! claim. The basis is measurement against the in-game Oilers roster, recorded in
//! docs/validation/nhl24-encoding-20261101.md.
use std::path::{Path, PathBuf};

use serde::Serialize;
use studio_domain::Platform;
use tdb_frostbite::{nhl24, oodle::Oodle};

use super::Registry;
use crate::dto::ProjectSummaryDto;
use crate::error::ApiError;

pub const GAME_ID: &str = "nhl24";

/// The install and decoder an open NHL 24 session saves through.
#[derive(Clone, Debug)]
pub(crate) struct Nhl24Binding {
    install: PathBuf,
    oodle: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct Nhl24Status {
    /// The install folder holding `game`, `game-UPDATE` and `game-mods`.
    pub install: String,
    /// Studio's overlay files currently in `game-mods`, by relative path.
    pub overlay_files: Vec<String>,
    /// Other files in `game-mods` (another mod's), which Studio never touches.
    pub other_mod_files: usize,
    /// The emulator executable that is running, if any (saves and restores wait for it).
    pub emulator_running: Option<String>,
    /// The project id of the open NHL 24 session on this install, if any.
    pub open_project: Option<u32>,
}

#[derive(Serialize)]
pub struct Nhl24Opened {
    pub summary: ProjectSummaryDto,
    /// The working copy the session edits (outside the game folders).
    pub db_path: String,
    /// True when Studio's overlay supplied the database (earlier saves are in effect).
    pub from_overlay: bool,
    pub status: Nhl24Status,
}

fn io(e: std::io::Error) -> ApiError {
    ApiError::Workspace(e.to_string())
}

fn roots(install: &Path) -> Result<nhl24::Roots, ApiError> {
    let r = nhl24::Roots::install(install);
    if !r.update.join(nhl24::GLOBALS_TOC).is_file() || !r.game.is_dir() {
        return Err(ApiError::Workspace(format!(
            "{} is not an NHL 24 install: choose the folder that holds game, game-UPDATE (01.60) and game-mods",
            install.display()
        )));
    }
    Ok(r)
}

fn canonical(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

/// Write `bytes` to `path` through a temporary file in the same folder.
fn replace(path: &Path, bytes: &[u8]) -> Result<(), ApiError> {
    use std::io::Write;
    let dir = path.parent().expect("working files have a parent");
    std::fs::create_dir_all(dir).map_err(io)?;
    let mut t = tempfile::NamedTempFile::new_in(dir).map_err(io)?;
    t.write_all(bytes).map_err(io)?;
    t.as_file().sync_all().map_err(io)?;
    t.persist(path).map_err(|e| io(e.error))?;
    Ok(())
}

impl Registry {
    fn nhl24_project_for(&self, install: &Path) -> Option<u32> {
        let key = canonical(install);
        self.nhl24
            .lock()
            .unwrap()
            .iter()
            .find(|(_, b)| canonical(&b.install) == key)
            .map(|(id, _)| *id)
    }

    pub(crate) fn nhl24_binding(&self, id: u32) -> Option<Nhl24Binding> {
        self.nhl24.lock().unwrap().get(&id).cloned()
    }

    pub(crate) fn forget_nhl24(&self, id: u32) {
        self.nhl24.lock().unwrap().remove(&id);
    }

    /// What Studio has placed in `game-mods`, and whether a save or restore can run now.
    pub fn nhl24_status(&self, install: &Path) -> Result<Nhl24Status, ApiError> {
        let r = roots(install)?;
        let view = nhl24::View::open(&r, true).map_err(io)?;
        let overlay_files = view.owned_files();
        let mut other = 0;
        if r.mods.is_dir() {
            for e in walk(&r.mods) {
                let rel = e
                    .strip_prefix(&r.mods)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if !overlay_files.contains(&rel) {
                    other += 1;
                }
            }
        }
        Ok(Nhl24Status {
            install: install.to_string_lossy().into_owned(),
            overlay_files,
            other_mod_files: other,
            emulator_running: nhl24::running_emulator(),
            open_project: self.nhl24_project_for(install),
        })
    }

    /// Open the database NHL 24 currently loads as an edit session.
    pub fn nhl24_open(
        &self,
        install: &Path,
        oodle: &Path,
        defs_root: &str,
    ) -> Result<Nhl24Opened, ApiError> {
        let r = roots(install)?;
        if let Some(id) = self.nhl24_project_for(install) {
            return Err(ApiError::Workspace(format!(
                "this NHL 24 install is already open as project {id}; close it first"
            )));
        }
        let decoder = Oodle::open(oodle).map_err(|e| {
            ApiError::Workspace(format!(
                "the Oodle decoder {} could not be loaded: {e}",
                oodle.display()
            ))
        })?;
        let current = nhl24::read_database(&r, Some(&decoder)).map_err(io)?;
        let work = r.state.join("working");
        let db_path = work.join("nhl_game.db");
        let meta_path = work.join("nhl_game-meta.xml");
        replace(&db_path, &current.db)?;
        replace(&meta_path, &current.meta_xml)?;
        let summary = self.open_workspace_from_paths(
            Some(GAME_ID),
            Platform::Ps4,
            &db_path.to_string_lossy(),
            None,
            &meta_path.to_string_lossy(),
            defs_root,
        )?;
        self.nhl24.lock().unwrap().insert(
            summary.project_id,
            Nhl24Binding {
                install: install.to_path_buf(),
                oodle: oodle.to_path_buf(),
            },
        );
        Ok(Nhl24Opened {
            db_path: db_path.to_string_lossy().into_owned(),
            from_overlay: current.from_overlay,
            status: self.nhl24_status(install)?,
            summary,
        })
    }

    /// Save an NHL 24 session: the overlay first, then the working copy and the baseline.
    pub(crate) fn nhl24_save(&self, id: u32, b: &Nhl24Binding) -> Result<u64, ApiError> {
        let r = roots(&b.install)?;
        let decoder = Oodle::open(&b.oodle).map_err(io)?;
        self.with_workspace(id, |ws| {
            let bytes = ws.primary().save().map_err(ApiError::Domain)?;
            let parsed = tdb_core::TdbFile::parse(&bytes)
                .map_err(|e| ApiError::Workspace(format!("the edited database does not parse: {e}")))?;
            if parsed.serialize() != bytes {
                return Err(ApiError::Workspace(
                    "the edited database does not round-trip; nothing was written".into(),
                ));
            }
            let stock = nhl24::stock(&r, Some(&decoder)).map_err(io)?;
            let cas = nhl24::choose_cas(&r).map_err(io)?;
            let plan = nhl24::plan(&stock, &bytes, cas).map_err(io)?;
            nhl24::apply(&r, &plan, nhl24::running_emulator).map_err(io)?;
            ws.save_in_place().map_err(|e| {
                ApiError::Workspace(format!(
                    "NHL 24 was updated in game-mods, but Studio's working copy was not saved ({e}); close and reopen NHL 24 before editing further"
                ))
            })
        })
    }

    /// Remove Studio's overlay from `game-mods`, returning the game to its own database.
    /// Refused while an NHL 24 session on this install is open.
    pub fn nhl24_restore(&self, install: &Path) -> Result<Nhl24Status, ApiError> {
        let r = roots(install)?;
        if let Some(id) = self.nhl24_project_for(install) {
            return Err(ApiError::Workspace(format!(
                "close NHL 24 (project {id}) before restoring the stock database"
            )));
        }
        nhl24::restore(&r, nhl24::running_emulator).map_err(io)?;
        self.nhl24_status(install)
    }
}

/// Every file under `dir` (bounded depth; `game-mods` is shallow).
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), 0)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && depth < 12 {
                stack.push((p, depth + 1));
            } else if p.is_file() {
                out.push(p);
            }
        }
    }
    out
}
