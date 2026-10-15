//! Save Editor — the launch screen's standalone tool: open one save container or extracted
//! database as its own edit session, with no project folder and no game installation.
//!
//! The file is identified against the bundled game definitions
//! ([`studio_domain::save_identify`]), opened with that target's bundled metadata (or, as an
//! explicit advanced choice, a metadata file the user supplies), and edited through the same
//! workspace and operations as Mod Studio. What differs is the boundary:
//!
//! * Saving writes back to the opened file only, after copying the original into the app data
//!   directory the first time a session overwrites it.
//! * Operations that write game files rather than the opened file (dictionary renames, settings
//!   tunables, archive members) are refused for these sessions. The bundled dictionary is attached
//!   for name resolution only.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use studio_domain::baselines::{self, BaselineStore};
use studio_domain::save_identify::{self, FileFormat, Identification, OpenPolicy, SchemaIndex};
use studio_domain::{EditSession, Platform, Project, ProjectRole, Workspace};

use super::{timestamp, Registry};
use crate::dto::{SaveCandidateDto, SaveEditorOpenDto, SaveEditorSaveDto, SaveFileIdentityDto};
use crate::error::ApiError;

/// A Save Editor session's file and its one-time backup.
pub(crate) struct SaveEditorFile {
    path: PathBuf,
    backup_dir: PathBuf,
    /// The copy of the original taken before the first overwrite in this session.
    backup: Option<PathBuf>,
    read_only: bool,
}

/// Display name of a platform.
pub(crate) fn platform_label(p: Platform) -> &'static str {
    match p {
        Platform::Ps3 => "PlayStation 3",
        Platform::Xbox => "Xbox 360",
        Platform::Ps4 => "PlayStation 4",
        Platform::Recomp => "Legacy Recompiled",
    }
}

/// `NHL 14 (PlayStation 3)` → `NHL 14`.
fn bare_title(title: &str) -> String {
    match title.rfind(" (") {
        Some(i) if title.ends_with(')') => title[..i].to_string(),
        _ => title.to_string(),
    }
}

fn format_label(id: &Identification) -> String {
    match (&id.probe.format, id.probe.container_version.as_deref()) {
        (FileFormat::Database, _) => "Extracted game database".into(),
        (FileFormat::Ps4Save, _) => "PS4 save data".into(),
        (FileFormat::Container, Some("v4")) => {
            "PS3 / Xbox 360 save (container v4, compressed)".into()
        }
        (FileFormat::Container, Some(v)) => format!("PS3 / Xbox 360 save (container {v})"),
        (FileFormat::Container, None) => "PS3 / Xbox 360 save".into(),
    }
}

fn identity_dto(id: &Identification) -> SaveFileIdentityDto {
    let top = id.candidates.first().map(|c| c.score);
    let (decision, reason) = match id.open_policy() {
        OpenPolicy::Allowed => ("allowed", None),
        OpenPolicy::Caution(r) => ("caution", Some(r)),
        OpenPolicy::Refused(r) => ("refused", Some(r)),
    };
    SaveFileIdentityDto {
        path: id.path.to_string_lossy().into_owned(),
        file_name: id
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        format: match id.probe.format {
            FileFormat::Database => "database",
            FileFormat::Container => "container",
            FileFormat::Ps4Save => "ps4_save",
        }
        .into(),
        format_label: format_label(id),
        kind: id.probe.kind.clone(),
        kind_label: save_identify::kind_label(&id.probe.kind).into(),
        container_version: id.probe.container_version.clone(),
        checksums_valid: id.probe.checksums_valid,
        writable: id.probe.writable,
        write_experimental: id.probe.write_experimental,
        change_list: id.change_list,
        game_confident: id.game_confident,
        platform_confident: id.platform_confident,
        candidates: id
            .candidates
            .iter()
            .map(|c| SaveCandidateDto {
                game_id: c.game_id.clone(),
                platform: c.platform.as_str().into(),
                title: bare_title(&c.title),
                platform_label: platform_label(c.platform).into(),
                exact_schema: c.exact_schema,
                evidence: c.evidence.clone(),
                leading: Some(c.score) == top,
            })
            .collect(),
        decision: decision.into(),
        decision_reason: reason,
        notes: id.notes(),
        path_hints: id.path_hints.clone(),
        own_tables: id.own_tables as u32,
        supplied_tables: id.supplied_tables as u32,
    }
}

/// The user-facing failure for a file that is not a readable database or save.
fn unreadable(path: &Path, e: impl std::fmt::Display) -> ApiError {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let multipart = name.to_ascii_uppercase().starts_with("DATA000");
    ApiError::Workspace(if multipart {
        format!(
            "{name} is one part of a PS3 save split across several DATA files. Save Editor does not read split saves yet."
        )
    } else {
        format!(
            "{name} could not be read as a game database or save ({e}). Choose the save's SYS-DATA file (PS3), the save file itself (Xbox 360), its DATA file (PS4), or an extracted database (.db)."
        )
    })
}

impl Registry {
    /// The bundled targets' schema evidence, built once per baseline inventory.
    fn schema_index(&self, store: &BaselineStore) -> Result<Arc<SchemaIndex>, ApiError> {
        let mut guard = self.schema_index.lock().unwrap();
        if let Some(ix) = guard
            .as_ref()
            .filter(|ix| ix.inventory_path() == store.inventory_path)
        {
            return Ok(ix.clone());
        }
        let ix = Arc::new(SchemaIndex::build(store).map_err(ApiError::Domain)?);
        *guard = Some(ix.clone());
        Ok(ix)
    }

    /// Identify a chosen file: what it is, which game and platform it belongs to, and whether
    /// Save Editor may open it. Reads only.
    pub fn save_editor_identify(
        &self,
        path: &str,
        store: &BaselineStore,
    ) -> Result<SaveFileIdentityDto, ApiError> {
        let p = Path::new(path);
        let id = self
            .schema_index(store)?
            .identify(p)
            .map_err(|e| unreadable(p, e))?;
        Ok(identity_dto(&id))
    }

    /// Open a file as a Save Editor session under the chosen `game_id`/`platform`.
    ///
    /// The file is identified again here (the client's earlier answer is not trusted): the chosen
    /// target must be one of its candidates, a refusal is final, and a cautioned file needs
    /// `acknowledged`. `meta_override` is the advanced path for a file no bundled definition
    /// describes (NHL 20 on PS4 has none): the supplied metadata must describe every table and
    /// field the file carries, and the game/platform are the user's explicit choice.
    #[allow(clippy::too_many_arguments)]
    pub fn save_editor_open(
        &self,
        path: &str,
        game_id: &str,
        platform: Platform,
        acknowledged: bool,
        meta_override: Option<&str>,
        store: &BaselineStore,
        defs_root: &str,
        backup_dir: &Path,
    ) -> Result<SaveEditorOpenDto, ApiError> {
        let p = Path::new(path);
        let id = self
            .schema_index(store)?
            .identify(p)
            .map_err(|e| unreadable(p, e))?;
        let file = identity_dto(&id);
        let manual = meta_override.map(str::trim).filter(|m| !m.is_empty());
        let target = store.target(game_id, platform);
        let candidate = id
            .candidates
            .iter()
            .find(|c| c.game_id == game_id && c.platform == platform);

        match id.open_policy() {
            // Only "no bundled definition describes it" yields to an explicit metadata choice.
            OpenPolicy::Refused(_)
                if manual.is_some() && id.candidates.is_empty() && !id.change_list => {}
            OpenPolicy::Refused(reason) => return Err(ApiError::Workspace(reason)),
            OpenPolicy::Caution(reason) if !acknowledged => {
                return Err(ApiError::Workspace(format!(
                    "{reason} Confirm that you understand before opening it."
                )))
            }
            _ => {}
        }

        let (meta_path, title) = match manual {
            Some(m) => {
                let bytes = std::fs::read(m).map_err(|source| ApiError::Io {
                    path: m.to_string(),
                    source,
                })?;
                let meta = save_identify::parse_meta(&bytes, m).map_err(ApiError::Domain)?;
                let sig = save_identify::file_signature(p).map_err(ApiError::Domain)?;
                if !save_identify::metadata_covers(&meta, &sig) {
                    return Err(ApiError::Workspace(
                        "The chosen metadata does not describe this file's database (a table or field is missing or has a different size). Choose the metadata that came with the same game and update.".into(),
                    ));
                }
                let title = target
                    .map(|t| bare_title(&t.title))
                    .unwrap_or_else(|| game_id.to_uppercase());
                (PathBuf::from(m), title)
            }
            None => {
                let c = candidate.ok_or_else(|| {
                    ApiError::Workspace(format!(
                        "This file was not identified as {game_id} on {}; choose one of the games it was identified as.",
                        platform_label(platform)
                    ))
                })?;
                let target = target.ok_or_else(|| {
                    ApiError::Workspace(format!(
                        "no bundled definitions for {game_id} on {}",
                        platform_label(platform)
                    ))
                })?;
                let meta = target.component(baselines::ROLE_META).ok_or_else(|| {
                    ApiError::Workspace(format!("no bundled metadata for {}", c.title))
                })?;
                // Refuse a damaged or substituted bundle rather than open with the wrong schema.
                store.read_verified(meta).map_err(ApiError::Domain)?;
                (store.component_path(meta), bare_title(&c.title))
            }
        };

        let role = if id.probe.format == FileFormat::Database {
            ProjectRole::Nhlng
        } else {
            ProjectRole::Roster
        };
        let mut project =
            Project::load(path, platform, role, &meta_path, defs_root).map_err(ApiError::Domain)?;
        // A save carries only part of the game database; the tables it lacks are shown read-only
        // from the target's bundled database (never written back). A bare database stands alone.
        let mut supplied = 0;
        if id.probe.format != FileFormat::Database {
            if let Some(main) = target.and_then(|t| t.component(baselines::ROLE_MAIN_DB)) {
                let base = store.read_verified(main).map_err(ApiError::Domain)?;
                supplied = project
                    .supplement_from(Arc::new(base))
                    .map_err(ApiError::Domain)?;
            }
        }
        let session = EditSession::begin(project);
        let mut ws = Workspace::single(session, path.to_string());
        ws.set_game_id(game_id);
        // The bundled dictionary resolves names that live there; it is never written (dictionary
        // edits are refused for Save Editor sessions).
        if let Some(d) = target.and_then(|t| t.component(baselines::ROLE_DICTIONARY)) {
            if store.read_verified(d).is_ok() {
                ws.attach_dictionary_at_open(&store.component_path(d).to_string_lossy());
            }
        }
        let read_only = !id.probe.writable;
        let summary = self.register(ws)?;
        self.save_editor.lock().unwrap().insert(
            summary.project_id,
            SaveEditorFile {
                path: p.to_path_buf(),
                backup_dir: backup_dir.to_path_buf(),
                backup: None,
                read_only,
            },
        );
        Ok(SaveEditorOpenDto {
            summary,
            file,
            game_id: game_id.into(),
            platform: platform.as_str().into(),
            title,
            platform_label: platform_label(platform).into(),
            read_only,
            supplied_tables: supplied as u32,
        })
    }

    /// Save a Save Editor session back to its file. The first save of a session copies the file
    /// as it is on disk into the backup directory before anything is written; the in-place save
    /// then refuses if the file changed since it was opened.
    pub fn save_editor_save(&self, id: u32) -> Result<SaveEditorSaveDto, ApiError> {
        let (path, backup_dir, needs_backup, read_only) = {
            let g = self.save_editor.lock().unwrap();
            let f = g.get(&id).ok_or_else(|| {
                ApiError::Workspace("this session was not opened by Save Editor".into())
            })?;
            (
                f.path.clone(),
                f.backup_dir.clone(),
                f.backup.is_none(),
                f.read_only,
            )
        };
        if read_only {
            return Err(ApiError::Workspace(
                "Studio cannot write this save format back, so this session is view-only.".into(),
            ));
        }
        let mut new_backup = None;
        // Even an unedited save is re-packaged (a compressed container is recompressed), so the
        // original is copied before the first write regardless of pending edits.
        if needs_backup {
            let dest = backup_path(&backup_dir, &path);
            copy_verified(&path, &dest)?;
            self.save_editor
                .lock()
                .unwrap()
                .entry(id)
                .and_modify(|f| f.backup = Some(dest.clone()));
            new_backup = Some(dest);
        }
        let files = self.save_workspace_unchecked(id)?;
        Ok(SaveEditorSaveDto {
            files_written: files as u32,
            backup_path: new_backup.map(|p| p.to_string_lossy().into_owned()),
            summary: self.summary(id)?,
        })
    }

    /// `true` when `id` is a Save Editor session.
    pub(crate) fn is_save_editor(&self, id: u32) -> bool {
        self.save_editor.lock().unwrap().contains_key(&id)
    }

    /// Refuse an operation that writes game files rather than the opened save.
    pub(crate) fn refuse_in_save_editor(&self, id: u32, what: &str) -> Result<(), ApiError> {
        if self.is_save_editor(id) {
            return Err(ApiError::Workspace(format!(
                "{what} changes the game's own files rather than this save, so it is not available in Save Editor."
            )));
        }
        Ok(())
    }

    pub(crate) fn forget_save_editor(&self, id: u32) {
        self.save_editor.lock().unwrap().remove(&id);
    }
}

/// `<backup_dir>/<stamp> <folder> - <file>` — the enclosing folder keeps `SYS-DATA` / `DATA`
/// backups from different saves apart.
fn backup_path(dir: &Path, file: &Path) -> PathBuf {
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_alphanumeric() || " -_.".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = file
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stamp = timestamp()
        .replace([':', 'T'], "-")
        .trim_end_matches('Z')
        .to_string();
    let base = if folder.is_empty() || folder == name {
        clean(&format!("{stamp} {name}"))
    } else {
        clean(&format!("{stamp} {folder} - {name}"))
    };
    let mut candidate = dir.join(&base);
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{base} ({n})"));
        n += 1;
    }
    candidate
}

fn copy_verified(from: &Path, to: &Path) -> Result<(), ApiError> {
    let io = |p: &Path| {
        let p = p.display().to_string();
        move |source| ApiError::Io {
            path: p.clone(),
            source,
        }
    };
    let bytes = std::fs::read(from).map_err(io(from))?;
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    std::fs::write(to, &bytes).map_err(io(to))?;
    if std::fs::read(to).map_err(io(to))? != bytes {
        return Err(ApiError::Workspace(format!(
            "the backup {} did not read back as the original; nothing was saved",
            to.display()
        )));
    }
    Ok(())
}
