use std::{fs, path::Path};
use crate::{dto::{ModLibraryDto, ModLibraryEntryDto}, error::ApiError};
use super::Registry;

fn error(message: impl ToString) -> ApiError { ApiError::Workspace(message.to_string()) }

impl Registry {
    /// Read-only library discovery. Invalid packages are reported individually; a bad download
    /// must not hide the rest of the library. Do not traverse links out of the selected folder.
    pub fn mod_library_scan(&self, root: &str) -> Result<ModLibraryDto, ApiError> {
        let root = Path::new(root);
        if !root.is_absolute() || !root.is_dir() { return Err(error("Choose an existing, absolute downloads folder.")); }
        let mut result = ModLibraryDto { mods: vec![], warnings: vec![] };
        self.scan_mod_folder(root, &mut result, 0)?;
        result.mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.path.cmp(&b.path)));
        Ok(result)
    }

    fn scan_mod_folder(&self, root: &Path, result: &mut ModLibraryDto, depth: usize) -> Result<(), ApiError> {
        if depth > 32 { result.warnings.push(format!("Skipped deeply nested folder: {}", root.display())); return Ok(()); }
        for entry in fs::read_dir(root).map_err(error)? {
            let entry = match entry { Ok(e) => e, Err(e) => { result.warnings.push(e.to_string()); continue; } };
            let path = entry.path();
            let meta = match fs::symlink_metadata(&path) { Ok(m) => m, Err(e) => { result.warnings.push(format!("{}: {e}", path.display())); continue; } };
            #[cfg(windows)]
            let linked = { use std::os::windows::fs::MetadataExt; meta.file_attributes() & 0x400 != 0 };
            #[cfg(not(windows))]
            let linked = false;
            if meta.file_type().is_symlink() || linked { continue; }
            if meta.is_dir() {
                if let Err(e) = self.scan_mod_folder(&path, result, depth + 1) { result.warnings.push(format!("{}: {e}", path.display())); }
            } else if path.extension().is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("nhlmod")) {
                match self.package_inspect(&path.to_string_lossy()) {
                    Ok(p) => result.mods.push(ModLibraryEntryDto { package_id: p.package_id, path: p.path, name: p.name, version: p.version,
                        author: p.author, description: p.description, game_id: p.game_id, created: p.created,
                        platforms: p.targets.into_iter().filter(|t| t.status == "supported").map(|t| t.platform).collect() }),
                    Err(e) => result.warnings.push(format!("Couldn’t read {}: {e}", path.display())),
                }
            }
        }
        Ok(())
    }

    /// Require the executable's own folder, not an emulator root or an ISO directory.
    pub fn mod_manager_check_target(&self, root: &str, platform: &str) -> Result<String, ApiError> {
        let extensions: &[&str] = match platform {
            "xbox" => &["xex"], "ps3" | "ps4" => &["bin", "elf"], "recomp" => &["exe", "elf"],
            _ => return Err(error("Choose a supported platform.")),
        };
        let path = Path::new(root);
        if !path.is_absolute() || !path.is_dir() { return Err(error("Choose an existing game installation folder.")); }
        let found = fs::read_dir(path).map_err(error)?.filter_map(Result::ok).any(|entry|
            entry.path().is_file() && entry.path().extension().is_some_and(|ext| extensions.iter().any(|allowed| ext.to_string_lossy().eq_ignore_ascii_case(allowed))));
        if !found { return Err(error(format!("Choose the folder containing the game's {} file.", extensions.iter().map(|x| format!(".{x}")).collect::<Vec<_>>().join(" or ")))); }
        Ok(root.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installation_folder_requires_platform_executable_in_selected_folder() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_string_lossy().into_owned();
        let registry = Registry::default();
        assert!(registry.mod_manager_check_target(&root, "xbox").is_err());
        fs::write(temp.path().join("DEFAULT.XEX"), b"test").unwrap();
        assert!(registry.mod_manager_check_target(&root, "xbox").is_ok());
        assert!(registry.mod_manager_check_target(&root, "ps3").is_err());
        fs::write(temp.path().join("EBOOT.BIN"), b"test").unwrap();
        assert!(registry.mod_manager_check_target(&root, "ps3").is_ok());
        assert!(registry.mod_manager_check_target(&root, "unknown").is_err());
    }
    #[test]
    fn library_scans_subfolders_and_reports_broken_downloads() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("NHL07")).unwrap();
        fs::write(temp.path().join("NHL07/broken.NHLMOD"), b"invalid").unwrap();
        fs::write(temp.path().join("readme.txt"), b"ignored").unwrap();
        let scanned = Registry::default().mod_library_scan(&temp.path().to_string_lossy()).unwrap();
        assert!(scanned.mods.is_empty());
        assert_eq!(scanned.warnings.len(), 1);
    }
}
