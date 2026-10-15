//! Portable developer projects: validate first, extract privately, publish to a new folder only.
use std::{collections::HashSet, fs, io::Read, path::{Path, PathBuf}};
use studio_domain::{project_store::{self as ps, ProjectManifest, ProjectStore}, tdb_txn::FileIdentity};
use crate::{dto::ProjectArchivePreviewDto, error::ApiError};
use super::Registry;
fn err(e: impl std::fmt::Display) -> ApiError { ApiError::Workspace(e.to_string()) }

// Apply Windows rules on every host so a shared project cannot contain aliases or drive paths.
fn relative(name: &str) -> Result<PathBuf, ApiError> {
    if name.is_empty() || name.contains('\\') || name.split('/').any(|part| {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        part.is_empty() || part == "." || part == ".." || part.ends_with([' ', '.']) ||
        part.chars().any(|c| c.is_control() || "<>:\"|?*".contains(c)) ||
        matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") ||
        (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    }) { return Err(err(format!("The archive contains an unsafe path: {name}"))); }
    Ok(PathBuf::from(name))
}

fn inspect(zip: &mut zip::ZipArchive<fs::File>) -> Result<(ProjectManifest, u64), ApiError> {
    if zip.len() > 100_000 { return Err(err("This project archive contains too many files.")); }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(err)?;
        let name = entry.name().trim_end_matches('/');
        relative(name)?;
        if !names.insert(name.to_lowercase()) { return Err(err("The archive contains duplicate file paths.")); }
        if entry.unix_mode().is_some_and(|m| !matches!(m & 0o170000, 0 | 0o100000 | 0o040000)) {
            return Err(err("Linked or special files cannot be imported."));
        }
        if name.eq_ignore_ascii_case(ps::LOCK_FILE) || name.eq_ignore_ascii_case("working/.tdb-txn/lock") || name.eq_ignore_ascii_case("working/.tdb-txn/ACTIVE") {
            return Err(err("This archive contains an active session or unfinished save. Save and close the original project, then package it again."));
        }
        total = total.checked_add(entry.size()).ok_or_else(|| err("Archive size is invalid."))?;
        if total > 128 * 1024 * 1024 * 1024 { return Err(err("This archive exceeds the 128 GB project import limit.")); }
    }
    let mut file = zip.by_name(ps::MANIFEST_FILE).map_err(|_| err("This is not a Studio developer project archive."))?;
    if file.size() > 4 * 1024 * 1024 { return Err(err("The project manifest is too large.")); }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(err)?;
    let manifest: ProjectManifest = serde_json::from_slice(&bytes).map_err(err)?;
    if manifest.schema != ps::MANIFEST_SCHEMA && manifest.schema != ps::MANIFEST_SCHEMA_V1 { return Err(err("This project requires a different Studio version.")); }
    let w = &manifest.working;
    for value in [&w.main_db, &w.meta, &w.dictionary, &w.roster_save, &w.roster_container_dir].into_iter().flatten() { relative(value)?; }
    for c in &manifest.source.components { relative(&c.baseline)?; }
    for c in &manifest.sources { relative(&c.relative)?; }
    for c in &manifest.staged_files { relative(&c.relative)?; }
    for c in &manifest.materialized_archives { relative(&c.relative)?; }
    for c in &manifest.member_edits { relative(&c.archive_relative)?; relative(&c.member_path)?; }
    let platform = crate::parse_platform(&manifest.platform)?;
    if studio_domain::shipping_target(&manifest.game_id, platform).is_none() { return Err(err("This project's game and platform are not supported.")); }
    Ok((manifest, total))
}

impl Registry {
    pub fn project_archive_preview(&self, archive: &str) -> Result<ProjectArchivePreviewDto, ApiError> {
        let mut zip = zip::ZipArchive::new(fs::File::open(archive).map_err(err)?).map_err(err)?;
        let (m, _) = inspect(&mut zip)?;
        Ok(ProjectArchivePreviewDto { name: m.details.name, game_id: m.game_id, platform: m.platform })
    }

    pub fn project_import_archive(&self, archive: &str, destination: &str, source_root: Option<&str>) -> Result<String, ApiError> {
        let mut zip = zip::ZipArchive::new(fs::File::open(archive).map_err(err)?).map_err(err)?;
        let (mut manifest, total) = inspect(&mut zip)?;
        let target = Path::new(destination);
        if !target.is_absolute() || target.exists() { return Err(err("Choose a new, absolute project folder. Existing folders are never overwritten.")); }
        let source = source_root.filter(|s| !s.trim().is_empty());
        if source.is_some_and(|s| !Path::new(s).is_absolute() || !Path::new(s).is_dir()) { return Err(err("Choose an existing game source folder.")); }
        let projects = self.projects.lock().unwrap();
        let mut guards = vec![("the archive", Path::new(archive))];
        if let Some(s) = source { guards.push(("the game source", Path::new(s))); }
        for binding in projects.bindings.values() { guards.push(("an open project", &binding.store.dir)); }
        ps::check_project_folder(target, &guards).map_err(err)?;
        let parent = target.parent().ok_or_else(|| err("Choose a project storage folder."))?;
        fs::create_dir_all(parent).map_err(err)?;
        super::project::check_free_space(parent, total.saturating_add(1024 * 1024), "Importing the project")?;
        let stage = tempfile::Builder::new().prefix(".nhlms-import-").tempdir_in(parent).map_err(err)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(err)?;
            let output = stage.path().join(relative(entry.name().trim_end_matches('/'))?);
            if entry.is_dir() { fs::create_dir_all(output).map_err(err)?; continue; }
            fs::create_dir_all(output.parent().unwrap()).map_err(err)?;
            let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&output).map_err(err)?;
            let expected = entry.size();
            let written = std::io::copy(&mut entry.by_ref().take(expected.saturating_add(1)), &mut file).map_err(err)?;
            if written != expected { return Err(err("A project file has an invalid size.")); }
            file.sync_all().map_err(err)?;
        }
        // Baselines and editable sources must match the identities shipped in the manifest.
        for (path, len, sha) in manifest.source.components.iter().map(|c| (&c.baseline, c.len, &c.sha256))
            .chain(manifest.sources.iter().map(|c| (&c.relative, c.len, &c.sha256))) {
            let actual = FileIdentity::of_file(&stage.path().join(path)).map_err(err)?;
            if actual.len != len || actual.sha256 != *sha { return Err(err(format!("Project file verification failed: {path}"))); }
        }
        // A shared archive must never retain someone else's live installation destinations.
        manifest.destinations = Default::default();
        if let Some(s) = source {
            manifest.destinations.picked_root = s.into();
            manifest.destinations.install_dir = Some(s.into());
            manifest.destinations.assets_dir = Some(s.into());
        }
        manifest.legacy_sidecar = None;
        manifest.deployments.clear();
        for c in &mut manifest.materialized_archives { c.destination = source.map(|s| Path::new(s).join(&c.relative).to_string_lossy().into_owned()).unwrap_or_default(); }
        let store = ProjectStore { dir: stage.path().into(), manifest };
        store.save_manifest().map_err(err)?;
        ProjectStore::open(stage.path()).map_err(err)?;
        if let Some(main) = &store.manifest.working.main_db {
            let bytes = fs::read(store.path(main)).map_err(err)?;
            studio_domain::Project::reparse_packaged(bytes, crate::parse_platform(&store.manifest.platform)?).map_err(err)?;
            let meta = store.manifest.working.meta.as_ref().ok_or_else(|| err("The project's database metadata is missing."))?;
            tdb_defs::FieldMetaIndex::from_xml(&fs::read_to_string(store.path(meta)).map_err(err)?).map_err(err)?;
        }
        // Directory rename publishes the complete validated tree in one operation. Windows
        // refuses an existing destination; the explicit check also gives a useful error.
        if target.exists() { return Err(err("The destination now exists. Choose another project folder.")); }
        fs::rename(stage.path(), target).map_err(err)?;
        Ok(target.to_string_lossy().into_owned())
    }
}
