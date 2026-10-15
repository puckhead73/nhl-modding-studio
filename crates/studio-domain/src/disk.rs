//! Free-space query for the volume holding a path (ONB-002: materialisation, deployment backups
//! and extraction refuse when the estimate does not fit). `None` when the answer is unavailable;
//! callers treat that as "unknown", never as "enough".

use std::path::{Path, PathBuf};

/// Bytes available to the current user on the volume holding `path` (or its nearest existing
/// ancestor).
pub fn free_space(path: &Path) -> Option<u64> {
    let mut probe: PathBuf = if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir().ok()?.join(path) };
    while !probe.exists() {
        probe = probe.parent()?.to_path_buf();
    }
    free_space_of_existing(&probe)
}

#[cfg(windows)]
fn free_space_of_existing(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut free_to_caller: u64 = 0;
    let mut total: u64 = 0;
    let mut free: u64 = 0;
    // SAFETY: `wide` is NUL-terminated and outlives the call; the out-pointers are valid u64s.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free_to_caller, &mut total, &mut free) };
    (ok != 0).then_some(free_to_caller)
}

#[cfg(not(windows))]
fn free_space_of_existing(_path: &Path) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_temp_volume_reports_a_figure_on_windows() {
        let f = super::free_space(&std::env::temp_dir().join("does-not-exist-yet/nor-this"));
        if cfg!(windows) {
            assert!(f.is_some_and(|b| b > 0));
        }
    }
}
