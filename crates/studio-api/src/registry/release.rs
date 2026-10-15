//! Saved-project release outputs. Build every eligible target before publishing any file;
//! developer archives stream to disk and are read back before publication.
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}};
use sha2::{Digest, Sha256};
use studio_domain::{BaselineStore, project_store::{ProjectStore, canonical_or_planned, is_within}, tdb_txn::FileIdentity};
use crate::{dto::{PackageBuildRequestDto, ProjectArchiveDto, ProjectBuildDto}, error::ApiError};
use super::Registry;

fn err(e: impl std::fmt::Display) -> ApiError { ApiError::Workspace(e.to_string()) }
fn path_text(p: &Path) -> String { p.to_string_lossy().into() }
fn file_name(s: &str) -> String {
    let name: String = s.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).collect();
    let name = name.trim_matches(|c: char| c == '.' || c.is_whitespace());
    if name.is_empty() { "Project".into() } else { name.into() }
}

fn output_dir(store: &ProjectStore, root: &str, child: &str) -> Result<PathBuf, ApiError> {
    let root = Path::new(root);
    if !root.is_absolute() { return Err(err("Choose an absolute projects storage folder in Settings → Paths.")); }
    let dir = root.join(child);
    store.refuse_guarded_write("Packaging the project", &dir)?;
    let resolved = canonical_or_planned(&dir).ok_or_else(|| err("The output folder cannot be resolved."))?;
    let project = fs::canonicalize(&store.dir).map_err(err)?;
    if is_within(&resolved, &project) { return Err(err("The packaging destination must be outside this project folder.")); }
    fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir)
}

impl Registry {
    /// Save-as copies the complete saved project to a new folder. The original remains intact;
    /// the caller switches sessions only after this verified copy succeeds.
    pub fn project_copy_to(&self, id: u32, destination: &str) -> Result<String, ApiError> {
        if self.has_session(id) && self.with_workspace(id, |w| Ok(w.is_dirty()))? {
            return Err(err("Save the project before changing its folder."));
        }
        let projects = self.projects.lock().unwrap();
        let store = &projects.bindings.get(&id).ok_or(ApiError::UnknownProject(id))?.store;
        let target = Path::new(destination);
        if !target.is_absolute() { return Err(err("Choose an absolute project folder path.")); }
        let mut roots = store.guarded_roots();
        roots.push(("the current project".into(), store.dir.clone()));
        for binding in projects.bindings.values() { roots.push(("an open project".into(), binding.store.dir.clone())); }
        let refs: Vec<_> = roots.iter().map(|(label, path)| (label.as_str(), path.as_path())).collect();
        studio_domain::project_store::check_project_folder(target, &refs).map_err(err)?;
        if target.exists() { return Err(err("Choose a new project folder. Existing folders are never overwritten.")); }
        let parent = target.parent().ok_or_else(|| err("Choose a project folder within a storage folder."))?;
        fs::create_dir_all(parent).map_err(err)?;
        let files = collect(&store.dir)?;
        super::project::check_free_space(parent, files.iter().map(|(_, _, i)| i.len).sum(), "Copying the project")?;
        let stage = tempfile::Builder::new().prefix(".nhlms-project-").tempdir_in(parent).map_err(err)?;
        for (relative, source, identity) in &files {
            let dest = stage.path().join(relative);
            fs::create_dir_all(dest.parent().unwrap()).map_err(err)?;
            fs::copy(source, &dest).map_err(err)?;
            if FileIdentity::of_file(&dest).map_err(err)? != *identity { return Err(err("A project file changed while copying. Save and try again.")); }
        }
        if collect(&store.dir)? != files { return Err(err("Project files changed while copying. Save and try again.")); }
        ProjectStore::open(stage.path()).map_err(err)?;
        // Reserve the final directory exclusively, then move only our staged children into it.
        fs::create_dir(target).map_err(err)?;
        for entry in fs::read_dir(stage.path()).map_err(err)? {
            let entry = entry.map_err(err)?;
            fs::rename(entry.path(), target.join(entry.file_name())).map_err(|e| err(format!("Could not finish copying to {}: {e}. The original project is unchanged.", target.display())))?;
        }
        Ok(path_text(target))
    }

    /// A build packages all supported content, without silent category exclusions. A platform
    /// that cannot consume all content is reported as skipped, never marked compatible.
    pub fn project_build(&self, id: u32, root: &str, inventory: Option<&BaselineStore>, defs: &str) -> Result<ProjectBuildDto, ApiError> {
        let inv = self.package_inventory(id, inventory, defs)?;
        if inv.dirty { return Err(err("Save the project before building.")); }
        let blocked: Vec<_> = inv.categories.iter().filter(|c| c.count > 0 && c.status == "blocked").flat_map(|c| c.blockers.iter().cloned()).collect();
        if !blocked.is_empty() { return Err(err(format!("This build contains content that cannot be packaged yet: {}", blocked.join("; ")))); }
        let include: Vec<_> = inv.categories.iter().filter(|c| c.count > 0).map(|c| c.category.clone()).collect();
        if include.is_empty() { return Err(err("There are no project changes to build.")); }
        let projects = self.projects.lock().unwrap();
        let store = &projects.bindings.get(&id).ok_or(ApiError::UnknownProject(id))?.store;
        let m = &store.manifest;
        let mut skipped = vec![];
        let targets: Vec<_> = inv.targets.iter().filter(|t| {
            // NHL15 PS4 is a different generation, even if a future metadata-only edit happens
            // to be portable. It is never automatically built with the PS3/360 version.
            if m.game_id == "nhl15" && (t.platform == "ps4") != (m.platform == "ps4") { return false; }
            if t.status == "supported" { true } else {
                skipped.push(format!("{}: {}", t.platform, t.components.iter().filter(|c| c.status == "unsupported").map(|c| c.reason.as_str()).collect::<Vec<_>>().join("; ")));
                false
            }
        }).collect();
        if targets.is_empty() { return Err(err(format!("No compatible build targets: {}", skipped.join("; ")))); }
        let dir = output_dir(store, root, "Builds")?;
        let name = if m.details.name.trim().is_empty() { store.dir.file_name().unwrap_or_default().to_string_lossy().into_owned() } else { m.details.name.clone() };
        let stem = file_name(&name);
        // Keep existing releases; choose the same numbered build name across all variants.
        let mut n = 1;
        let paths = loop {
            let name = if n == 1 { stem.clone() } else { format!("{stem} ({n})") };
            let paths: Vec<_> = targets.iter().map(|t| dir.join(format!("{}_{}_{}.nhlmod", name, if t.platform == "xbox" { "xbox360" } else { &t.platform }, file_name(&m.game_id)))).collect();
            if paths.iter().all(|p| !p.exists()) { break paths; }
            n += 1;
        };
        let stage = tempfile::Builder::new().prefix(".nhlms-build-").tempdir_in(&dir).map_err(err)?;
        let mut packages = vec![];
        for (target, output) in targets.iter().zip(&paths) {
            let staged = stage.path().join(output.file_name().unwrap());
            packages.push(Self::build_saved_package(store, &PackageBuildRequestDto {
                name: name.clone(), version: if m.details.version.is_empty() { "1.0.0".into() } else { m.details.version.clone() },
                description: m.details.description.clone(), include: include.clone(), exclude: vec![],
                platforms: vec![target.platform.clone()], out_path: Some(path_text(&staged)),
            }, inventory, defs)?);
        }
        // Publish without overwriting an existing release. Any failure
        // is reported with paths already published; completed packages remain recoverable.
        let mut published = vec![];
        for (package, output) in packages.iter_mut().zip(&paths) {
            tempfile::TempPath::try_from_path(&package.path).map_err(err)?.persist_noclobber(output).map_err(|e| err(format!("Could not publish {}: {e}. Completed outputs: {}", output.display(), published.join(", "))))?;
            package.path = path_text(output);
            published.push(package.path.clone());
        }
        Ok(ProjectBuildDto { packages, skipped })
    }

    pub fn project_archive(&self, id: u32, root: &str) -> Result<ProjectArchiveDto, ApiError> {
        if self.has_session(id) && self.with_workspace(id, |w| Ok(w.is_dirty()))? { return Err(err("Save the project before packaging it.")); }
        // Save and source-import operations use this lock too: keep the saved tree stable while
        // it is collected. Changes by external programs are detected by file identities below.
        let projects = self.projects.lock().unwrap();
        let store = &projects.bindings.get(&id).ok_or(ApiError::UnknownProject(id))?.store;
        let dir = output_dir(store, root, "Project Archives")?;
        let name = if store.manifest.details.name.is_empty() { store.dir.file_name().unwrap_or_default().to_string_lossy().into_owned() } else { store.manifest.details.name.clone() };
        let stem = file_name(&name);
        let mut output = dir.join(format!("{stem}.nhlproject"));
        let mut n = 2;
        while output.exists() { output = dir.join(format!("{stem} ({n}).nhlproject")); n += 1; }
        let files = collect(&store.dir)?;
        let total: u64 = files.iter().map(|(_, _, identity)| identity.len).sum();
        super::project::check_free_space(&dir, total.saturating_add(total / 100).saturating_add(1024 * 1024), "Packaging the project")?;
        let mut temporary = tempfile::NamedTempFile::new_in(&dir).map_err(err)?;
        {
            let mut zip = zip::ZipWriter::new(temporary.as_file_mut());
            for (relative, path, identity) in &files {
                let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(identity.len >= u32::MAX as u64);
                zip.start_file(relative, options).map_err(err)?;
                let mut source = fs::File::open(path).map_err(err)?;
                let mut hash = Sha256::new();
                let mut buffer = vec![0u8; 1024 * 1024];
                let mut len = 0;
                loop {
                    let count = source.read(&mut buffer).map_err(err)?;
                    if count == 0 { break; }
                    zip.write_all(&buffer[..count]).map_err(err)?;
                    hash.update(&buffer[..count]); len += count as u64;
                }
                if len != identity.len || format!("{:x}", hash.finalize()) != identity.sha256 { return Err(err(format!("{relative} changed while packaging. Save and try again."))); }
            }
            zip.finish().map_err(err)?.sync_all().map_err(err)?;
        }
        // Independent archive read verifies every payload (including ZIP CRCs) and source hash.
        let mut zip = zip::ZipArchive::new(fs::File::open(temporary.path()).map_err(err)?).map_err(err)?;
        for (name, _, identity) in &files {
            let mut entry = zip.by_name(name).map_err(err)?;
            let mut hash = Sha256::new();
            let mut buf = vec![0; 1024 * 1024];
            loop { let n = entry.read(&mut buf).map_err(err)?; if n == 0 { break; } hash.update(&buf[..n]); }
            if format!("{:x}", hash.finalize()) != identity.sha256 { return Err(err(format!("Archive verification failed for {name}."))); }
        }
        drop(zip);
        if collect(&store.dir)? != files { return Err(err("Project files changed during packaging. Save and try again.")); }
        let identity = FileIdentity::of_file(temporary.path()).map_err(err)?;
        temporary.persist_noclobber(&output).map_err(err)?;
        Ok(ProjectArchiveDto { path: path_text(&output), files: files.len() as u64, len: identity.len, sha256: identity.sha256 })
    }
}

fn collect(root: &Path) -> Result<Vec<(String, PathBuf, FileIdentity)>, ApiError> {
    fn visit(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf, FileIdentity)>) -> Result<(), ApiError> {
        for entry in fs::read_dir(dir).map_err(err)? {
            let entry = entry.map_err(err)?;
            let path = entry.path();
            let rel = path.strip_prefix(root).map_err(err)?.to_string_lossy().replace('\\', "/");
            if rel == "working/.tdb-txn/ACTIVE" { return Err(err("The project's last save was interrupted. Recover it before copying or packaging the project.")); }
            if rel == studio_domain::project_store::LOCK_FILE || rel == "working/.tdb-txn/lock" { continue; }
            let meta = fs::symlink_metadata(&path).map_err(err)?;
            #[cfg(windows)]
            let reparse = { use std::os::windows::fs::MetadataExt; meta.file_attributes() & 0x400 != 0 };
            #[cfg(not(windows))]
            let reparse = false;
            if meta.file_type().is_symlink() || reparse { return Err(err(format!("{rel} is a linked file or folder. Copy its contents into the project before packaging."))); }
            if meta.is_dir() { visit(root, &path, out)?; }
            else if meta.is_file() { out.push((rel, path.clone(), FileIdentity::of_file(&path).map_err(err)?)); }
            else { return Err(err(format!("{rel} is not a regular project file."))); }
        }
        Ok(())
    }
    let mut out = vec![];
    visit(root, root, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}
