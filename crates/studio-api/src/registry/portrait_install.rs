//! In-app download + install of the supplementary portrait database (the operator-compiled
//! `graphical database/portraits` tree with its `portraits.sqlite` index), published as one 7z
//! archive on Google Drive.
//!
//! One background job at a time: **download** (resumable `Range` fetch into `<base>/downloads/`,
//! progress in bytes) and then **install** (7z extraction into a sibling temp folder, the folder
//! holding `portraits.sqlite` moved into place as `<base>/graphical database/portraits`, progress
//! in files). `<base>` is the folder beside the historical corpus — the bundle root in a packaged
//! Studio — so the library is found by the shipped-layout rule without any override. A manually
//! chosen archive takes the install half alone. Cancel stops either half; a cancelled download
//! keeps its `.part` file and resumes next time. Status is polled, never pushed.
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::dto::{PortraitInstallPlanDto, PortraitInstallStatusDto};
use crate::error::ApiError;
use super::Registry;

/// The published archive (Google Drive file id) and what it is.
pub const DRIVE_FILE_ID: &str = "1xHzPyKXA60adnHpnrNQyn9Y7Kj7ppRd4";
pub const ARCHIVE_NAME: &str = "NHL Modding Studio - Portrait Database.7z";
/// Size measured on 21 Oct 2026 (`Content-Range` of the published file). Display only; the
/// download trusts the server's figure.
pub const EXPECTED_BYTES: u64 = 31_737_672_102;
const LIBRARY_DIR: &[&str] = &["graphical database", "portraits"];
const INDEX_FILE: &str = "portraits.sqlite";

fn download_url() -> String {
    format!("https://drive.usercontent.google.com/download?id={DRIVE_FILE_ID}&export=download&confirm=t")
}

#[derive(Clone, Default)]
pub struct Progress {
    /// `idle` | `downloading` | `extracting` | `done` | `failed` | `cancelled`.
    pub phase: String,
    pub message: String,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub files_done: u64,
    pub archive_path: Option<String>,
    pub install_root: Option<String>,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Default)]
pub struct PortraitInstallState {
    progress: Mutex<Progress>,
    cancel: AtomicBool,
    running: AtomicBool,
    /// Set by the worker when a library was installed; consumed by the registry on the next status
    /// poll to drop its cached portrait backend (the worker has no registry handle).
    library_changed: AtomicBool,
}

impl PortraitInstallState {
    fn set<F: FnOnce(&mut Progress)>(&self, f: F) {
        f(&mut self.progress.lock().unwrap());
    }
    fn snapshot(&self) -> Progress {
        self.progress.lock().unwrap().clone()
    }
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

fn now() -> String {
    studio_domain::project_store::now_iso()
}

fn text(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Where the download and the library go for this Studio: beside the open corpus, else beside the
/// bundled corpus under the resource dir.
fn base_dir(registry: &Registry, resource_dir: &Path) -> PathBuf {
    if let Some(dir) = registry.corpus_dir.lock().unwrap().clone() {
        if let Some(parent) = dir.parent() {
            return parent.to_path_buf();
        }
    }
    resource_dir.to_path_buf()
}

fn library_root(base: &Path) -> PathBuf {
    LIBRARY_DIR.iter().fold(base.to_path_buf(), |p, s| p.join(s))
}

/// `Err(reason)` when the worker must stop.
type Step<T> = Result<T, String>;

/// One request's worth of the file. Bounded `Range` requests are served as bytes even when Drive
/// answers an open-ended download with its "quota exceeded" / "can't scan" page, so the archive
/// is fetched in bounded chunks, each retried with backoff.
const CHUNK: u64 = 32 * 1024 * 1024;

fn is_quota_page(page: &str) -> bool {
    page.contains("Quota exceeded") || page.contains("quota exceeded")
}

fn download(state: &PortraitInstallState, dest_dir: &Path) -> Step<PathBuf> {
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("cannot create {}: {e}", dest_dir.display()))?;
    let final_path = dest_dir.join(ARCHIVE_NAME);
    let part_path = dest_dir.join(format!("{ARCHIVE_NAME}.part"));
    if final_path.is_file() {
        let len = std::fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);
        state.set(|p| {
            p.bytes_done = len;
            p.bytes_total = Some(len);
            p.message = "A complete download is already present; installing it.".into();
        });
        return Ok(final_path);
    }
    let mut written = std::fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0);
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(Some(std::time::Duration::from_secs(600)))
        .user_agent(concat!("NHL-Modding-Studio/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part_path)
        .map_err(|e| format!("cannot open {}: {e}", part_path.display()))?;
    let mut total: Option<u64> = None;
    let mut url = download_url();
    state.set(|p| {
        p.bytes_done = written;
        p.message = if written > 0 { "Resuming the download…".into() } else { "Connecting…".into() };
    });
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if let Some(t) = total {
            if written >= t {
                break;
            }
        }
        if state.cancelled() {
            return Err("cancelled".into());
        }
        let end = match total {
            Some(t) => (written + CHUNK - 1).min(t - 1),
            None => written + CHUNK - 1,
        };
        // One chunk, retried: Drive's interstitial pages are intermittent and the quota page
        // clears with time; a dropped connection resumes at `written`.
        let mut attempt = 0u32;
        let (mut body, chunk_total, chunk_len) = loop {
            attempt += 1;
            if state.cancelled() {
                return Err("cancelled".into());
            }
            let sent = client
                .get(&url)
                .header(reqwest::header::RANGE, format!("bytes={written}-{end}"))
                .send();
            let r = match sent {
                Ok(r) => r,
                Err(e) if attempt < 6 => {
                    state.set(|p| p.message = format!("Connection problem ({e}); retrying ({attempt})…"));
                    std::thread::sleep(std::time::Duration::from_secs(3 * attempt as u64));
                    continue;
                }
                Err(e) => return Err(format!("the download stopped: {e} (it resumes from where it stopped next time)")),
            };
            let status = r.status();
            let content_type = r.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
            let content_range = r.headers().get(reqwest::header::CONTENT_RANGE).and_then(|v| v.to_str().ok()).map(str::to_string);
            let content_length = r.content_length();
            if status.as_u16() == 206 && !content_type.starts_with("text/html") {
                let t = content_range.as_deref().and_then(|s| s.rsplit('/').next()).and_then(|t| t.parse::<u64>().ok());
                break (r, t, content_length);
            }
            if status.as_u16() == 416 {
                // Nothing left: the partial file already holds everything.
                if let Some(t) = total {
                    if written >= t {
                        break (r, Some(t), Some(0));
                    }
                }
                return Err("the download server rejected the resume position; delete the .part file and start again".into());
            }
            if !status.is_success() {
                return Err(format!("the download server answered {status}"));
            }
            let page = r.text().unwrap_or_default();
            if is_quota_page(&page) {
                if attempt >= 6 {
                    return Err("Google Drive's download quota for this file is exhausted for now (its \"Quota exceeded\" page); try again later, or download it in a browser and use Install from archive".into());
                }
                state.set(|p| p.message = format!("Google Drive reports its download quota is exceeded; waiting and retrying ({attempt})…"));
                std::thread::sleep(std::time::Duration::from_secs(20 * attempt as u64));
                continue;
            }
            if let Some(next) = interstitial_form_url(&page) {
                url = next;
            }
            if attempt >= 6 {
                return Err("the download link kept answering with a page instead of the file; try again later, or download it in a browser and use Install from archive".into());
            }
            state.set(|p| p.message = format!("Google Drive answered with a page instead of the file; retrying ({attempt})…"));
            std::thread::sleep(std::time::Duration::from_secs(3 * attempt as u64));
        };
        if total.is_none() {
            total = chunk_total;
            if let Some(t) = total {
                let needed = t.saturating_sub(written);
                if let Some(free) = studio_domain::free_space(dest_dir) {
                    if free < needed.saturating_add(256 * 1024 * 1024) {
                        return Err(format!(
                            "the download needs {} GiB more on the volume holding {} but only {} GiB is free",
                            needed / (1 << 30),
                            dest_dir.display(),
                            free / (1 << 30)
                        ));
                    }
                }
            }
            state.set(|p| {
                p.bytes_total = total;
                p.message = "Downloading…".into();
            });
        }
        let expect = chunk_len.unwrap_or(end - written + 1);
        let mut got = 0u64;
        while got < expect {
            if state.cancelled() {
                let _ = file.flush();
                return Err("cancelled".into());
            }
            let n = match body.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    // Keep what arrived; the next chunk request resumes at `written`.
                    let _ = file.flush();
                    state.set(|p| p.message = format!("The connection dropped ({e}); resuming…"));
                    break;
                }
            };
            file.write_all(&buf[..n]).map_err(|e| format!("cannot write {}: {e}", part_path.display()))?;
            written += n as u64;
            got += n as u64;
            state.set(|p| p.bytes_done = written);
        }
        if got == 0 {
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);
    let total = total.ok_or("the download never reported the file size")?;
    if written != total {
        return Err(format!("the download ended early ({written} of {total} bytes); it resumes from where it stopped next time"));
    }
    state.set(|p| p.message = "Download complete; verifying…".into());
    std::fs::rename(&part_path, &final_path).map_err(|e| format!("cannot place {}: {e}", final_path.display()))?;
    Ok(final_path)
}


/// The download URL named by Google Drive's "can't scan for viruses" interstitial: its form's
/// `action` plus every hidden input (`id`, `export`, `confirm`, `uuid`, …). `None` when the page
/// carries no such form.
fn interstitial_form_url(page: &str) -> Option<String> {
    let attr = |tag: &str, name: &str| -> Option<String> {
        let key = format!("{name}=\"");
        let i = tag.find(&key)? + key.len();
        let rest = &tag[i..];
        let j = rest.find('"')?;
        Some(rest[..j].to_string())
    };
    let form_start = page.find("<form")?;
    let form_end = page[form_start..].find("</form>").map(|e| form_start + e).unwrap_or(page.len());
    let form = &page[form_start..form_end];
    let form_tag_end = form.find('>')?;
    let action = attr(&form[..form_tag_end], "action")?;
    if !action.starts_with("https://") || !action.contains("google.com") {
        return None;
    }
    let mut params: Vec<(String, String)> = Vec::new();
    let mut rest = form;
    while let Some(i) = rest.find("<input") {
        let tag_end = rest[i..].find('>').map(|e| i + e).unwrap_or(rest.len());
        let tag = &rest[i..tag_end];
        if let (Some(n), Some(v)) = (attr(tag, "name"), attr(tag, "value")) {
            params.push((n, v));
        }
        rest = &rest[tag_end..];
    }
    if params.is_empty() {
        return None;
    }
    let query: Vec<String> = params
        .iter()
        .map(|(n, v)| format!("{}={}", n, v.replace('&', "%26").replace(' ', "%20")))
        .collect();
    Some(format!("{action}?{}", query.join("&")))
}

/// Reject archive member names that could escape the destination.
fn safe_relative(name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() || p.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
        return None;
    }
    Some(p.to_path_buf())
}

fn extract(state: &PortraitInstallState, archive: &Path, base: &Path) -> Step<PathBuf> {
    let root = library_root(base);
    let parent = root.parent().ok_or("no library parent")?.to_path_buf();
    std::fs::create_dir_all(&parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    let archive_len = std::fs::metadata(archive).map(|m| m.len()).map_err(|e| format!("{}: {e}", archive.display()))?;
    if let Some(free) = studio_domain::free_space(&parent) {
        let needed = archive_len + archive_len / 8;
        if free < needed {
            return Err(format!(
                "installing needs about {} GiB free on the volume holding {} but only {} GiB is free",
                needed / (1 << 30),
                parent.display(),
                free / (1 << 30)
            ));
        }
    }
    let temp = parent.join("portraits.installing");
    if temp.exists() {
        std::fs::remove_dir_all(&temp).map_err(|e| format!("cannot clear {}: {e}", temp.display()))?;
    }
    std::fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
    state.set(|p| {
        p.phase = "extracting".into();
        p.files_done = 0;
        p.message = "Extracting the portrait archive…".into();
    });
    let mut files = 0u64;
    let mut cancelled = false;
    let result = sevenz_rust2::decompress_file_with_extract_fn(archive, &temp, |entry, reader, _| {
        if state.cancelled() {
            cancelled = true;
            return Ok(false);
        }
        let Some(rel) = safe_relative(&entry.name) else {
            return Err(sevenz_rust2::Error::Other(format!("unsafe member path {:?}", entry.name).into()));
        };
        let dest = temp.join(rel);
        if entry.is_directory {
            std::fs::create_dir_all(&dest).map_err(|e| sevenz_rust2::Error::Io(e, dest.to_string_lossy().to_string().into()))?;
            return Ok(true);
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| sevenz_rust2::Error::Io(e, parent.to_string_lossy().to_string().into()))?;
        }
        let mut out = std::fs::File::create(&dest).map_err(|e| sevenz_rust2::Error::Io(e, dest.to_string_lossy().to_string().into()))?;
        std::io::copy(reader, &mut out).map_err(|e| sevenz_rust2::Error::Io(e, dest.to_string_lossy().to_string().into()))?;
        files += 1;
        if files % 50 == 0 {
            state.set(|p| p.files_done = files);
        }
        Ok(true)
    });
    state.set(|p| p.files_done = files);
    if cancelled {
        let _ = std::fs::remove_dir_all(&temp);
        return Err("cancelled".into());
    }
    result.map_err(|e| {
        let _ = std::fs::remove_dir_all(&temp);
        format!("the archive could not be extracted: {e}")
    })?;
    // The folder that holds the index is the library, wherever the archive nested it.
    let library = find_index_dir(&temp, 0).unwrap_or_else(|| single_child_chain(&temp));
    if root.exists() {
        let replaced = parent.join(format!("portraits.replaced-{}", now().replace([':', '.'], "-")));
        std::fs::rename(&root, &replaced).map_err(|e| format!("cannot set aside the existing library {}: {e}", root.display()))?;
    }
    std::fs::rename(&library, &root).map_err(|e| format!("cannot place the library at {}: {e}", root.display()))?;
    if temp.exists() {
        let _ = std::fs::remove_dir_all(&temp);
    }
    Ok(root)
}

fn find_index_dir(dir: &Path, depth: usize) -> Option<PathBuf> {
    if dir.join(INDEX_FILE).is_file() {
        return Some(dir.to_path_buf());
    }
    if depth >= 4 {
        return None;
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    entries.sort();
    entries.iter().find_map(|d| find_index_dir(d, depth + 1))
}

/// With no index to anchor on, descend while a folder has exactly one child folder and no files.
fn single_child_chain(dir: &Path) -> PathBuf {
    let mut cur = dir.to_path_buf();
    loop {
        let Ok(rd) = std::fs::read_dir(&cur) else { return cur };
        let items: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        if items.len() == 1 && items[0].is_dir() {
            cur = items[0].clone();
        } else {
            return cur;
        }
    }
}

fn run(state: Arc<PortraitInstallState>, archive: Option<PathBuf>, base: PathBuf, delete_archive_after: bool) {
    let outcome: Step<PathBuf> = (|| {
        let archive = match archive {
            Some(a) => a,
            None => download(&state, &base.join("downloads"))?,
        };
        state.set(|p| p.archive_path = Some(text(&archive)));
        let root = extract(&state, &archive, &base)?;
        if delete_archive_after {
            let _ = std::fs::remove_file(&archive);
        }
        Ok(root)
    })();
    match outcome {
        Ok(root) => {
            state.library_changed.store(true, Ordering::SeqCst);
            state.set(|p| {
                p.phase = "done".into();
                p.install_root = Some(text(&root));
                p.message = "The portrait database is installed.".into();
                p.finished_at = Some(now());
            });
        }
        Err(e) if e == "cancelled" => state.set(|p| {
            p.phase = "cancelled".into();
            p.message = "Cancelled.".into();
            p.finished_at = Some(now());
        }),
        Err(e) => state.set(|p| {
            p.phase = "failed".into();
            p.error = Some(e.clone());
            p.message = e;
            p.finished_at = Some(now());
        }),
    }
    state.running.store(false, Ordering::SeqCst);
}

impl Registry {
    fn portrait_install_begin(&self, archive: Option<PathBuf>, resource_dir: &Path, delete_archive_after: bool) -> Result<PortraitInstallStatusDto, ApiError> {
        let state = self.portrait_install.clone();
        if state.running.swap(true, Ordering::SeqCst) {
            return Err(ApiError::Workspace("a portrait database download or install is already running".into()));
        }
        state.cancel.store(false, Ordering::SeqCst);
        let base = base_dir(self, resource_dir);
        state.set(|p| {
            *p = Progress {
                phase: if archive.is_some() { "extracting".into() } else { "downloading".into() },
                message: if archive.is_some() { "Preparing to extract…".into() } else { "Connecting…".into() },
                archive_path: archive.as_ref().map(|a| text(a)),
                install_root: Some(text(&library_root(&base))),
                started_at: Some(now()),
                ..Progress::default()
            };
        });
        std::thread::Builder::new()
            .name("portrait-install".into())
            .spawn(move || run(state, archive, base, delete_archive_after))
            .map_err(|e| ApiError::Workspace(format!("cannot start the install thread: {e}")))?;
        self.portrait_install_status()
    }

    /// Start the background download of the published archive and install it when complete.
    pub fn portrait_download_start(&self, resource_dir: &Path) -> Result<PortraitInstallStatusDto, ApiError> {
        self.portrait_install_begin(None, resource_dir, true)
    }

    /// Install a portrait archive the operator already has (`.7z`); the archive is kept.
    pub fn portrait_install_archive(&self, archive: &str, resource_dir: &Path) -> Result<PortraitInstallStatusDto, ApiError> {
        let p = PathBuf::from(archive.trim());
        if !p.is_file() {
            return Err(ApiError::Workspace(format!("{} is not a file", p.display())));
        }
        let mut magic = [0u8; 6];
        let ok = std::fs::File::open(&p).and_then(|mut f| f.read_exact(&mut magic)).is_ok() && magic == [0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c];
        if !ok {
            return Err(ApiError::Workspace(format!("{} is not a 7z archive; the portrait database ships as {ARCHIVE_NAME}", p.display())));
        }
        self.portrait_install_begin(Some(p), resource_dir, false)
    }

    /// Ask the running job to stop. A download keeps its partial file for resume.
    pub fn portrait_install_cancel(&self) -> Result<PortraitInstallStatusDto, ApiError> {
        self.portrait_install.cancel.store(true, Ordering::SeqCst);
        self.portrait_install_status()
    }

    /// The job's progress. Also the point where an installed library replaces the cached one.
    pub fn portrait_install_status(&self) -> Result<PortraitInstallStatusDto, ApiError> {
        if self.portrait_install.library_changed.swap(false, Ordering::SeqCst) {
            // The library landed in the shipped layout: clear any explicit override and reopen.
            let mut sidecar = self.read_portrait_sidecar();
            if sidecar.root.is_some() {
                sidecar.root = None;
                let _ = self.write_portrait_sidecar(&sidecar);
            }
            self.portraits.lock().unwrap().reset();
        }
        let p = self.portrait_install.snapshot();
        Ok(PortraitInstallStatusDto {
            phase: if p.phase.is_empty() { "idle".into() } else { p.phase },
            message: p.message,
            bytes_done: p.bytes_done,
            bytes_total: p.bytes_total,
            files_done: p.files_done,
            archive_path: p.archive_path,
            install_root: p.install_root,
            error: p.error,
            started_at: p.started_at,
            finished_at: p.finished_at,
        })
    }

    /// Where a download would go, where the library would be installed, and whether one is there.
    pub fn portrait_install_plan(&self, resource_dir: &Path) -> Result<PortraitInstallPlanDto, ApiError> {
        let base = base_dir(self, resource_dir);
        let root = library_root(&base);
        let download_dir = base.join("downloads");
        let partial = download_dir.join(format!("{ARCHIVE_NAME}.part"));
        let complete = download_dir.join(ARCHIVE_NAME);
        Ok(PortraitInstallPlanDto {
            download_dir: text(&download_dir),
            install_root: text(&root),
            installed: root.join(INDEX_FILE).is_file(),
            archive_name: ARCHIVE_NAME.into(),
            expected_bytes: EXPECTED_BYTES,
            partial_bytes: std::fs::metadata(&partial).map(|m| m.len()).unwrap_or(0),
            downloaded_archive: complete.is_file().then(|| text(&complete)),
            free_bytes: studio_domain::free_space(&base),
            source_url: format!("https://drive.google.com/file/d/{DRIVE_FILE_ID}/view"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_drive_interstitial_form_is_followed() {
        let page = r#"<html><body><form id="download-form" action="https://drive.usercontent.google.com/download" method="get">
          <input type="hidden" name="id" value="ABC"><input type="hidden" name="export" value="download">
          <input type="hidden" name="confirm" value="t"><input type="hidden" name="uuid" value="u-1"></form></body></html>"#;
        assert_eq!(
            interstitial_form_url(page).as_deref(),
            Some("https://drive.usercontent.google.com/download?id=ABC&export=download&confirm=t&uuid=u-1")
        );
        assert!(interstitial_form_url("<html>no form</html>").is_none());
        assert!(interstitial_form_url(r#"<form action="https://evil.example/x"><input name="a" value="b"></form>"#).is_none());
    }

    #[test]
    fn a_nested_archive_installs_as_the_library_root_and_sets_aside_an_existing_one() {
        let dir = tempfile::tempdir().unwrap();
        // A source tree the way the published archive nests it: <top>/portraits/{portraits.sqlite, aa/x.png}.
        let src = dir.path().join("src");
        let lib = src.join("NHL Modding Studio - Portrait Database").join("portraits");
        std::fs::create_dir_all(lib.join("aa")).unwrap();
        std::fs::write(lib.join(INDEX_FILE), b"sqlite-bytes").unwrap();
        std::fs::write(lib.join("aa").join("aaron_1.png"), b"png-bytes").unwrap();
        let archive = dir.path().join("portraits.7z");
        sevenz_rust2::compress_to_path(&src, &archive).unwrap();
        // An existing library at the destination is set aside, not deleted.
        let base = dir.path().join("base");
        let existing = library_root(&base);
        std::fs::create_dir_all(&existing).unwrap();
        std::fs::write(existing.join("old.txt"), b"old").unwrap();

        let state = PortraitInstallState::default();
        let root = extract(&state, &archive, &base).unwrap();
        assert_eq!(root, existing);
        assert_eq!(std::fs::read(root.join(INDEX_FILE)).unwrap(), b"sqlite-bytes");
        assert_eq!(std::fs::read(root.join("aa").join("aaron_1.png")).unwrap(), b"png-bytes");
        assert!(!root.join("old.txt").exists());
        let set_aside: Vec<_> = std::fs::read_dir(root.parent().unwrap()).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.starts_with("portraits.replaced-")).collect();
        assert_eq!(set_aside.len(), 1, "the previous library was set aside");
        assert!(!root.parent().unwrap().join("portraits.installing").exists(), "no temp folder left");
        assert_eq!(state.snapshot().files_done, 2);
        assert!(reg_archive_is_refused_when_cancelled(&archive, dir.path()));
    }

    fn reg_archive_is_refused_when_cancelled(archive: &Path, work: &Path) -> bool {
        // A cancelled extraction leaves nothing behind.
        let state = PortraitInstallState::default();
        state.cancel.store(true, Ordering::SeqCst);
        let base = work.join("cancelled-base");
        let r = extract(&state, archive, &base);
        r == Err("cancelled".into()) && !library_root(&base).exists() && !base.join("graphical database").join("portraits.installing").exists()
    }

    #[test]
    fn member_paths_cannot_escape_the_destination() {
        assert_eq!(safe_relative("portraits/a/b.png"), Some(PathBuf::from("portraits/a/b.png")));
        assert!(safe_relative("../x.png").is_none());
        assert!(safe_relative("a/../../x.png").is_none());
        assert!(safe_relative("C:/x.png").is_none() || cfg!(not(windows)));
        assert!(safe_relative("/etc/passwd").is_none());
    }

    #[test]
    fn the_library_is_the_folder_holding_the_index_wherever_the_archive_nested_it() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("NHL Modding Studio - Portrait Database").join("portraits");
        std::fs::create_dir_all(nested.join("aa")).unwrap();
        std::fs::write(nested.join(INDEX_FILE), b"x").unwrap();
        assert_eq!(find_index_dir(dir.path(), 0), Some(nested.clone()));
        // Without an index, descend the single-child chain and stop at the first branching folder.
        let dir2 = tempfile::tempdir().unwrap();
        let chain = dir2.path().join("top").join("portraits");
        std::fs::create_dir_all(chain.join("aa")).unwrap();
        std::fs::create_dir_all(chain.join("ab")).unwrap();
        assert_eq!(find_index_dir(dir2.path(), 0), None);
        assert_eq!(single_child_chain(dir2.path()), chain);
    }

    #[test]
    fn the_plan_reports_partial_and_complete_downloads_and_the_install_root() {
        let dir = tempfile::tempdir().unwrap();
        let reg = Registry::default();
        let p = reg.portrait_install_plan(dir.path()).unwrap();
        assert_eq!(p.install_root, library_root(dir.path()).to_string_lossy());
        assert!(!p.installed && p.partial_bytes == 0 && p.downloaded_archive.is_none());
        let dl = dir.path().join("downloads");
        std::fs::create_dir_all(&dl).unwrap();
        std::fs::write(dl.join(format!("{ARCHIVE_NAME}.part")), vec![0u8; 1234]).unwrap();
        assert_eq!(reg.portrait_install_plan(dir.path()).unwrap().partial_bytes, 1234);
        std::fs::create_dir_all(library_root(dir.path())).unwrap();
        std::fs::write(library_root(dir.path()).join(INDEX_FILE), b"x").unwrap();
        assert!(reg.portrait_install_plan(dir.path()).unwrap().installed);
        // A non-7z file is refused before any thread starts.
        let bogus = dir.path().join("bogus.7z");
        std::fs::write(&bogus, b"not an archive").unwrap();
        assert!(reg.portrait_install_archive(&bogus.to_string_lossy(), dir.path()).is_err());
        assert_eq!(reg.portrait_install_status().unwrap().phase, "idle");
    }
}
