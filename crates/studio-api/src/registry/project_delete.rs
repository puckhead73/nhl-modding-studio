use super::Registry;
use crate::error::ApiError;
use std::path::Path;
use studio_domain::project_store::{check_project_folder, is_within, ProjectLock, ProjectManifest, LOCK_FILE, MANIFEST_FILE, MANIFEST_SCHEMA, MANIFEST_SCHEMA_V1};

fn error(message: impl ToString) -> ApiError { ApiError::Workspace(message.to_string()) }

// Inspect the whole tree before deletion; never traverse linked folders or another project.
fn inspect_tree(root: &Path, path: &Path) -> Result<(), ApiError> {
    let metadata = std::fs::symlink_metadata(path).map_err(error)?;
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let linked = metadata.file_type().is_symlink();
    if linked || !is_within(&std::fs::canonicalize(path).map_err(error)?, root) {
        return Err(error("This project contains a linked file or folder. Remove the link before deleting the project."));
    }
    if metadata.is_dir() {
        if path != root && path.join(MANIFEST_FILE).exists() {
            return Err(error("This folder contains another Studio project. Move that project out before deleting this one."));
        }
        for entry in std::fs::read_dir(path).map_err(error)? { inspect_tree(root, &entry.map_err(error)?.path())?; }
    }
    Ok(())
}

impl Registry {
    /// Called only after the user confirms deletion in the recent-project list.
    /// References in the manifest (game sources, saves and released builds) are never followed.
    pub fn project_delete(&self, project_dir: &str) -> Result<(), ApiError> {
        let path = Path::new(project_dir);
        if !path.is_absolute() || !path.is_dir() { return Err(error("Choose an existing project folder.")); }
        let root = check_project_folder(path, &[])?;
        if root.parent().is_none() || root.file_name().is_none() { return Err(error("A drive root cannot be deleted as a project.")); }
        let projects = self.projects.lock().unwrap();
        if projects.bindings.values().any(|b| std::fs::canonicalize(&b.store.dir).is_ok_and(|p| is_within(&p, &root) || is_within(&root, &p))) {
            return Err(error("Close this project in Studio before deleting it."));
        }
        let manifest_path = root.join(MANIFEST_FILE);
        let manifest: ProjectManifest = serde_json::from_slice(&std::fs::read(&manifest_path).map_err(|_| error("This folder is not a recognized Studio project. Use Forget to remove it from the list."))?).map_err(error)?;
        if manifest.schema != MANIFEST_SCHEMA && manifest.schema != MANIFEST_SCHEMA_V1 {
            return Err(error("This project format is not supported for deletion by this Studio version."));
        }
        inspect_tree(&root, &root)?;
        let lock = ProjectLock::acquire(&root)?;
        // Keep the manifest and exclusive lock until the project contents are removed. A
        // failure retains the recent entry and reports the error instead of claiming success.
        for entry in std::fs::read_dir(&root).map_err(error)? {
            let entry = entry.map_err(error)?;
            if entry.file_name() == LOCK_FILE || entry.file_name() == MANIFEST_FILE { continue; }
            let target = entry.path();
            inspect_tree(&root, &target)?;
            let result = if entry.file_type().map_err(error)?.is_dir() { std::fs::remove_dir_all(&target) } else { std::fs::remove_file(&target) };
            result.map_err(|e| error(format!("Deletion could not finish. Some project files may already have been removed: {e}")))?;
        }
        std::fs::remove_file(manifest_path).map_err(error)?;
        drop(lock);
        let lock_path = root.join(LOCK_FILE);
        if lock_path.exists() { std::fs::remove_file(lock_path).map_err(error)?; }
        std::fs::remove_dir(root).map_err(error)?;
        Ok(())
    }
}
