//! Per-project profile — the saved settings for one game install, persisted as a sidecar
//! (`.nhlmodproj`) in the resolved per-game install dir. One file per install dir, so two NHL games
//! sharing a single RPCS3 root never collide.
//!
//! The profile records every file the project resolved to (including the roster save), so the
//! follow-up roster↔nhlng.db sync phase can reopen the full file set without re-running discovery.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::discovery::{DbPaths, InstallReport, SaveSlot};

/// The sidecar filename written into the install dir.
pub const SIDECAR_NAME: &str = ".nhlmodproj";

/// Current on-disk schema version (bump when the shape changes incompatibly).
pub const SCHEMA: u32 = 1;

/// The persisted per-project profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectProfile {
    pub schema: u32,
    pub game_id: String,
    pub platform: String,
    /// The folder the user originally pointed at.
    pub picked_root: String,
    /// The resolved per-game install dir (where this sidecar lives).
    pub install_dir: String,
    pub db_paths: DbPaths,
    pub saves: Vec<SaveSlot>,
    /// PS4: the matched game folder and the save-data location, chosen independently.
    #[serde(default)]
    pub game_dir: Option<String>,
    #[serde(default)]
    pub save_root: Option<String>,
    /// Whether the user has confirmed the game's big files are extracted.
    #[serde(default)]
    pub big_files_extracted: bool,
    /// Whether the user has been shown / answered the big-files prompt at least once.
    #[serde(default)]
    pub big_files_acknowledged: bool,
    /// Open-ended per-project preferences bag (future use).
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
}

impl ProjectProfile {
    /// The sidecar path for a given install dir.
    pub fn sidecar_path(install_dir: &Path) -> PathBuf {
        install_dir.join(SIDECAR_NAME)
    }

    /// Load the sidecar profile from an install dir, or `None` if absent / unreadable / malformed.
    pub fn load(install_dir: &Path) -> Option<ProjectProfile> {
        let text = std::fs::read_to_string(Self::sidecar_path(install_dir)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Write this profile to its install dir's sidecar (pretty JSON).
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::sidecar_path(Path::new(&self.install_dir));
        let text = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, text)
    }

    /// Build a profile from a fresh install report, carrying forward any preferences / big-files
    /// state from an existing sidecar in the same install dir.
    pub fn from_report(report: &InstallReport, big_files_extracted: bool) -> Option<Self> {
        let install_dir = report.install_dir.clone()?;
        let existing = ProjectProfile::load(Path::new(&install_dir));
        Some(ProjectProfile {
            schema: SCHEMA,
            game_id: report.game_id.clone(),
            platform: report.platform.clone(),
            picked_root: report.picked_root.clone(),
            install_dir,
            db_paths: report.db_paths.clone(),
            saves: report.saves.clone(),
            game_dir: report.game_dir.clone(),
            save_root: report.save_root.clone(),
            big_files_extracted,
            big_files_acknowledged: true,
            settings: existing.map(|e| e.settings).unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{DbPaths, InstallReport, SaveSlot};

    fn report_in(dir: &Path) -> InstallReport {
        InstallReport {
            game_id: "nhl14".into(),
            platform: "ps3".into(),
            picked_root: dir.to_string_lossy().into(),
            install_dir: Some(dir.to_string_lossy().into()),
            db_paths: DbPaths {
                nhlng: Some(dir.join("nhlng.db").to_string_lossy().into()),
                localization: None,
                meta_xml: Some(dir.join("nhlng-meta.xml").to_string_lossy().into()),
            },
            saves: vec![SaveSlot {
                kind: "roster".into(),
                path: dir.join("SYS-DATA").to_string_lossy().into(),
                container_version: "v4".into(),
                writable: true,
                profile: None,
                title_id: None,
                problem: None,
            }],
            minimum_met: true,
            extract_needed: false,
            game_dir: None,
            save_root: None,
            game_candidates: Vec::new(),
            problems: Vec::new(),
        }
    }

    #[test]
    fn sidecar_round_trips_and_carries_settings_forward() {
        let dir = std::env::temp_dir().join(format!("nhlms-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // First open: write a sidecar, then stash a user preference into it.
        let mut p = ProjectProfile::from_report(&report_in(&dir), false).unwrap();
        p.settings.insert("theme".into(), "slate".into());
        p.save().unwrap();

        let loaded = ProjectProfile::load(&dir).expect("sidecar loads");
        assert_eq!(loaded.game_id, "nhl14");
        assert_eq!(loaded.saves.len(), 1);
        assert_eq!(
            loaded.settings.get("theme").map(String::as_str),
            Some("slate")
        );

        // Re-open with big files now extracted: settings must carry forward from the old sidecar.
        let reopened = ProjectProfile::from_report(&report_in(&dir), true).unwrap();
        assert!(reopened.big_files_extracted);
        assert!(reopened.big_files_acknowledged);
        assert_eq!(
            reopened.settings.get("theme").map(String::as_str),
            Some("slate")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
