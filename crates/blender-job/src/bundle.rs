//! Locating (and later, installing) the pinned Blender build.
//!
//! Resolution order:
//! 1. An explicit user-configured `blender.exe` path (Settings) — used as-is, with a
//!    version-mismatch warning surfaced by the caller.
//! 2. The managed install at `<managed_root>/blender/<PINNED_VERSION>/` (populated by
//!    the on-demand download, milestone M3).
//! 3. Neither → [`LocateError::NotInstalled`] carrying user-facing instructions.
//!
//! The download itself is *not* in this crate (no network dependency here): the app
//! layer downloads the official blender.org archive and hands the file to
//! `verify_archive_sha256` + extraction (M3). Shrinkwrap output depends on the Blender
//! build, so the golden tests and the pin move together — never bump one without the
//! other.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The pinned Blender series. Exact archive + SHA256 land with the M3 download step;
/// the *series* gate is what `version_matches` enforces for bring-your-own installs.
pub const PINNED_VERSION: &str = "4.5";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlenderLocation {
    /// User-configured executable (may be any version — caller surfaces the warning).
    UserConfigured { exe: PathBuf, version_ok: bool },
    /// The managed, pinned install.
    Managed { exe: PathBuf },
}

impl BlenderLocation {
    pub fn exe(&self) -> &Path {
        match self {
            BlenderLocation::UserConfigured { exe, .. } | BlenderLocation::Managed { exe } => exe,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LocateError {
    #[error("configured Blender executable does not exist: {0}")]
    ConfiguredMissing(PathBuf),
    #[error(
        "Blender {PINNED_VERSION} is not installed. The donor-morph path needs headless \
         Blender: let the Studio download the pinned build (~a few hundred MB, one time), \
         or point Settings at an existing Blender {PINNED_VERSION} installation."
    )]
    NotInstalled,
}

/// Expected managed executable path under a managed root.
pub fn managed_exe(managed_root: &Path) -> PathBuf {
    let exe = if cfg!(windows) {
        "blender.exe"
    } else {
        "blender"
    };
    managed_root.join("blender").join(PINNED_VERSION).join(exe)
}

/// Resolve which Blender to run. `configured` is the Settings override, if any.
pub fn locate(
    configured: Option<&Path>,
    managed_root: &Path,
) -> Result<BlenderLocation, LocateError> {
    if let Some(exe) = configured {
        if !exe.is_file() {
            return Err(LocateError::ConfiguredMissing(exe.to_path_buf()));
        }
        let version_ok = query_version(exe)
            .map(|v| version_matches(&v))
            .unwrap_or(false);
        return Ok(BlenderLocation::UserConfigured {
            exe: exe.to_path_buf(),
            version_ok,
        });
    }
    let exe = managed_exe(managed_root);
    if exe.is_file() {
        return Ok(BlenderLocation::Managed { exe });
    }
    Err(LocateError::NotInstalled)
}

/// `blender --version` first line, e.g. `Blender 4.5.1`.
pub fn query_version(exe: &Path) -> Option<String> {
    let out = Command::new(exe).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().next().map(str::trim).map(str::to_owned)
}

/// Does a `blender --version` banner belong to the pinned series?
pub fn version_matches(banner: &str) -> bool {
    banner
        .strip_prefix("Blender ")
        .is_some_and(|v| v.starts_with(PINNED_VERSION))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_banner_matching() {
        assert!(version_matches("Blender 4.5.1"));
        assert!(version_matches("Blender 4.5"));
        assert!(!version_matches("Blender 4.2.0"));
        assert!(!version_matches("Blender 5.0.0"));
        assert!(!version_matches("nonsense"));
    }

    #[test]
    fn locate_prefers_configured_and_reports_missing() {
        let tmp = std::env::temp_dir().join(format!("bj_locate_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        // no configured, empty managed root → NotInstalled
        assert!(matches!(locate(None, &tmp), Err(LocateError::NotInstalled)));
        // configured but missing → ConfiguredMissing
        let ghost = tmp.join("nope").join("blender.exe");
        assert!(matches!(
            locate(Some(&ghost), &tmp),
            Err(LocateError::ConfiguredMissing(_))
        ));
        // managed present → Managed
        let exe = managed_exe(&tmp);
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, b"stub").unwrap();
        assert!(matches!(
            locate(None, &tmp),
            Ok(BlenderLocation::Managed { .. })
        ));
        std::fs::remove_dir_all(&tmp).ok();
    }
}
