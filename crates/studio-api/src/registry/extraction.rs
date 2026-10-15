//! ONB-002 — the extraction service surface of the [`Registry`]: read-only preflight, one
//! background job per destination with progress and cancellation, and the result the UI shows.
//! Both onboarding and Settings → Tools come through here.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use studio_domain::extract_service::{self, Progress};
use studio_domain::project_store::{canonical_or_planned, is_within, shown};
use studio_domain::Platform;

use super::Registry;
use crate::dto::{ExtractArchiveDto, ExtractCollisionDto, ExtractJobDto, ExtractPreflightDto};
use crate::error::ApiError;

pub(crate) struct Job {
    root: PathBuf,
    destination: PathBuf,
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<Progress>>,
    /// `running` | `cancelling` | `done` | `cancelled` | `failed`.
    state: Arc<Mutex<(String, Option<String>)>>,
}

#[derive(Default)]
pub(crate) struct ExtractionState {
    jobs: HashMap<u32, Job>,
    next: u32,
}

fn s(p: &Path) -> String {
    shown(p).to_string_lossy().into_owned()
}

impl Registry {
    /// Prepare a PS4 base package with a separately installed CLI helper. The
    /// common job surface keeps cancellation and destination exclusion visible.
    pub fn pkg_extract_start(&self, image:&str, root:&str, game_id:&str, extractor:&str) -> Result<ExtractJobDto,ApiError> {
        let root_path=canonical_or_planned(Path::new(root)).ok_or_else(||ApiError::Workspace("Package destination cannot be resolved.".into()))?;
        if root_path.exists() || !root_path.parent().is_some_and(Path::is_dir) {
            return Err(ApiError::Workspace("Choose a new package output folder with an existing parent; existing folders are never overwritten.".into()));
        }
        for b in self.projects.lock().unwrap().bindings.values() {
            if canonical_or_planned(&b.store.dir).is_some_and(|pd|is_within(&root_path,&pd)) {
                return Err(ApiError::Workspace("Extract game files outside the Studio project folder.".into()));
            }
        }
        let mut st=self.extractions.lock().unwrap();
        if st.jobs.values().any(|j|matches!(j.state.lock().unwrap().0.as_str(),"running"|"cancelling") && [&j.root, &j.destination].iter().any(|other| is_within(&root_path, other) || is_within(other, &root_path))) {
            return Err(ApiError::Workspace("An extraction already uses this destination; wait for it or cancel it.".into()));
        }
        let id=st.next;st.next+=1;
        let cancel=Arc::new(AtomicBool::new(false));
        let progress=Arc::new(Mutex::new(Progress::default()));
        let state=Arc::new(Mutex::new(("running".into(),None)));
        st.jobs.insert(id,Job {destination:root_path.clone(),root:root_path.clone(),cancel:cancel.clone(),progress:progress.clone(),state:state.clone()});
        drop(st);
        let image=PathBuf::from(image);let extractor=PathBuf::from(extractor);let game_id=game_id.to_string();
        let worker_state=state.clone();
        let spawned=std::thread::Builder::new().name(format!("nhlms-pkg-{id}")).spawn(move|| {
            let result=studio_domain::pkg_extract::extract(&image,&root_path,&extractor,&game_id,&cancel,&mut |message|progress.lock().unwrap().current=message.to_string());
            match result {
                Ok(out)=>{
                    let mut p=progress.lock().unwrap();p.files_written=out.files;p.bytes_written=out.bytes;
                    p.current=format!("Package files ready in {}. Extract the game archives next.",out.game_dir.display());
                    drop(p);*worker_state.lock().unwrap()=("done".into(),None);
                },
                Err(e)=>*worker_state.lock().unwrap()=(if cancel.load(Ordering::Relaxed){"cancelled"}else{"failed"}.into(),Some(e.to_string())),
            }
        });
        if let Err(e)=spawned {*state.lock().unwrap()=("failed".into(),Some(format!("Could not start package extraction: {e}")));}
        self.extract_status(id)
    }

    /// Materialize a disc into a new or empty game folder. The shared copier validates
    /// the ISO and publishes only complete output, making cancellation retryable.
    pub fn disc_extract_start(&self, image: &str, root: &str) -> Result<ExtractJobDto, ApiError> {
        let root_path = canonical_or_planned(Path::new(root)).ok_or_else(|| ApiError::Workspace("disc destination cannot be resolved".into()))?;
        {
            let projects = self.projects.lock().unwrap();
            for b in projects.bindings.values() {
                if canonical_or_planned(&b.store.dir).is_some_and(|pd| is_within(&root_path,&pd)) {
                    return Err(ApiError::Workspace("Extract game files outside the Studio project folder.".into()));
                }
            }
        }
        let image = PathBuf::from(image);
        // The worker validates the complete XDVDFS or ISO9660 manifest before
        // creating any output. Keep format acceptance in that shared copier;
        // the Xbox-only container probe would incorrectly reject PS3 discs.
        let mut st = self.extractions.lock().unwrap();
        if st.jobs.values().any(|j| matches!(j.state.lock().unwrap().0.as_str(), "running" | "cancelling")
            && [&j.root, &j.destination].iter().any(|other| is_within(&root_path, other) || is_within(other, &root_path))) {
            return Err(ApiError::Workspace("An extraction already uses this destination; wait for it or cancel it.".into()));
        }
        let id = st.next;
        st.next += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(Mutex::new(Progress::default()));
        let state = Arc::new(Mutex::new(("running".to_string(),None)));
        st.jobs.insert(id,Job { destination:root_path.clone(),root:root_path.clone(),cancel:cancel.clone(),progress:progress.clone(),state:state.clone() });
        drop(st);
        let worker_state = state.clone();
        let spawn = std::thread::Builder::new().name(format!("nhlms-disc-{id}")).spawn(move || {
            let result = studio_domain::disc_extract::extract(&image,&root_path,&cancel,&mut |done,total| {
                let mut p = progress.lock().unwrap();
                p.current = format!("Copying disc: {done} / {total} bytes");
                p.bytes_written = done;
            });
            let mut state = worker_state.lock().unwrap();
            match result {
                Ok(receipt) => {
                    let mut p = progress.lock().unwrap();
                    p.files_written = receipt.members as u64;
                    p.current = "Disc copied. Game archives can now be extracted.".into();
                    state.0 = "done".into();
                }
                Err(e) => {
                    state.0 = if cancel.load(Ordering::Relaxed) { "cancelled" } else { "failed" }.into();
                    state.1 = Some(e.to_string());
                }
            }
        });
        if let Err(e) = spawn {
            *state.lock().unwrap() = ("failed".into(),Some(format!("could not start disc extraction: {e}")));
        }
        self.extract_status(id)
    }

    /// Preflight a game installation's archives without writing anything.
    pub fn extract_preflight(&self, root: &str, platform: Platform) -> Result<ExtractPreflightDto, ApiError> {
        self.extract_preflight_to(root, platform, None)
    }

    pub fn extract_preflight_to(&self, root: &str, platform: Platform, destination: Option<&str>) -> Result<ExtractPreflightDto, ApiError> {
        let root_path = Path::new(root);
        if !root_path.is_dir() {
            return Err(ApiError::Workspace(format!("{root} is not a folder")));
        }
        // Never into any open project's folder: extraction is the installation-write exception,
        // not a project write.
        {
            let projects = self.projects.lock().unwrap();
            if let (Some(target), true) = (canonical_or_planned(root_path), !projects.bindings.is_empty()) {
                for b in projects.bindings.values() {
                    if let Some(pd) = canonical_or_planned(&b.store.dir) {
                        if is_within(&target, &pd) {
                            return Err(ApiError::Workspace(format!("{root} is inside the project folder {}; extraction targets the game installation only", s(&pd))));
                        }
                    }
                }
            }
        }
        let p = extract_service::preflight_to(root_path, Path::new(destination.unwrap_or(root)), platform);
        Ok(ExtractPreflightDto {
            root: s(&p.root),
            state: p.state.into(),
            reason: p.reason,
            archives: p
                .archives
                .iter()
                .map(|a| ExtractArchiveDto {
                    path: s(&a.path),
                    format: a.format.clone(),
                    members: a.members.len() as u64,
                    present: a.members.iter().filter(|m| m.state == "present").count() as u64,
                    state: a.state.into(),
                    note: a.note.clone(),
                })
                .collect(),
            members_total: p.members_total,
            members_present: p.members_present,
            members_to_write: p.members_to_write,
            bytes_to_write: p.bytes_to_write,
            free_bytes: p.free_bytes,
            collisions: p.collisions.iter().map(|c| ExtractCollisionDto { member: c.member.clone(), archives: c.archives.iter().map(|a| s(a)).collect() }).collect(),
        })
    }

    /// Start extraction into `root`. Refused while a job already runs for the same destination.
    pub fn extract_start(&self, root: &str, platform: Platform) -> Result<ExtractJobDto, ApiError> {
        self.extract_start_to(root, platform, None)
    }

    pub fn extract_start_to(&self, root: &str, platform: Platform, destination: Option<&str>) -> Result<ExtractJobDto, ApiError> {
        let root_path = canonical_or_planned(Path::new(root)).ok_or_else(|| ApiError::Workspace(format!("{root} cannot be resolved")))?;
        if !root_path.is_dir() {
            return Err(ApiError::Workspace(format!("{root} is not a folder")));
        }
        let _ = self.extract_preflight(root, platform)?; // the project-folder refusal
        let destination = canonical_or_planned(Path::new(destination.unwrap_or(root))).ok_or_else(|| ApiError::Workspace("Assets destination cannot be resolved".into()))?;
        for b in self.projects.lock().unwrap().bindings.values() {
            if let Some(pd) = canonical_or_planned(&b.store.dir) {
                if is_within(&destination, &pd) || is_within(&pd, &destination) { return Err(ApiError::Workspace("Choose an assets folder outside the project folder".into())); }
            }
        }
        let mut st = self.extractions.lock().unwrap();
        if let Some((id, _)) = st.jobs.iter().find(|(_, j)| {
            let running = matches!(j.state.lock().unwrap().0.as_str(), "running" | "cancelling");
            running && [&j.root, &j.destination].iter().any(|other| [&root_path, &destination].iter().any(|path| is_within(path, other) || is_within(other, path)))
        }) {
            return Err(ApiError::Workspace(format!("an extraction (job {id}) is already running for {}; wait for it or cancel it", s(&root_path))));
        }
        let id = st.next;
        st.next += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(Mutex::new(Progress::default()));
        let state = Arc::new(Mutex::new(("running".to_string(), None::<String>)));
        let job = Job { destination: destination.clone(), root: root_path.clone(), cancel: cancel.clone(), progress: progress.clone(), state: state.clone() };
        st.jobs.insert(id, job);
        drop(st);
        let root_for_thread = root_path.clone();
        std::thread::Builder::new()
            .name(format!("nhlms-extract-{id}"))
            .spawn(move || {
                let mut report = |p: &Progress| {
                    *progress.lock().unwrap() = p.clone();
                };
                let result = extract_service::extract_to(&root_for_thread, &destination, platform, &cancel, &mut report);
                let mut st = state.lock().unwrap();
                match result {
                    Ok(p) => {
                        *progress.lock().unwrap() = p.clone();
                        st.0 = if p.cancelled { "cancelled".into() } else { "done".into() };
                    }
                    Err(e) => {
                        st.0 = "failed".into();
                        st.1 = Some(e.to_string());
                    }
                }
            })
            .map_err(|e| ApiError::Workspace(format!("could not start the extraction thread: {e}")))?;
        self.extract_status(id)
    }

    pub fn extract_status(&self, job: u32) -> Result<ExtractJobDto, ApiError> {
        let st = self.extractions.lock().unwrap();
        let j = st.jobs.get(&job).ok_or_else(|| ApiError::Workspace(format!("no extraction job {job}")))?;
        let p = j.progress.lock().unwrap().clone();
        let (state, error) = j.state.lock().unwrap().clone();
        Ok(ExtractJobDto {
            destination: s(&j.destination),
            current_files_done: p.current_files_done,
            current_files_total: p.current_files_total,
            files_done: p.files_done,
            files_total: p.files_total,
            job,
            root: s(&j.root),
            state,
            current: p.current,
            archives_done: p.archives_done,
            archives_total: p.archives_total,
            files_written: p.files_written,
            files_retained: p.files_retained,
            files_conflicting: p.files_conflicting,
            files_blocked: p.files_blocked,
            files_unsupported: p.files_unsupported,
            files_unsafe: p.files_unsafe,
            archives_failed: p.archives_failed,
            bytes_written: p.bytes_written,
            problems: p.problems,
            error,
        })
    }

    /// Ask a running job to stop after the member it is writing. Owned outputs stay; nothing
    /// pre-existing is removed.
    pub fn extract_cancel(&self, job: u32) -> Result<ExtractJobDto, ApiError> {
        {
            let st = self.extractions.lock().unwrap();
            let j = st.jobs.get(&job).ok_or_else(|| ApiError::Workspace(format!("no extraction job {job}")))?;
            j.cancel.store(true, Ordering::Relaxed);
            let mut state = j.state.lock().unwrap();
            if state.0 == "running" {
                state.0 = "cancelling".into();
            }
        }
        self.extract_status(job)
    }

    /// The running job for a destination, if any (so a second screen can attach to it).
    pub fn extract_running_for(&self, root: &str) -> Option<ExtractJobDto> {
        let root_path = canonical_or_planned(Path::new(root))?;
        let id = {
            let st = self.extractions.lock().unwrap();
            st.jobs.iter().find(|(_, j)| matches!(j.state.lock().unwrap().0.as_str(), "running" | "cancelling") && is_within(&j.root, &root_path) && is_within(&root_path, &j.root)).map(|(id, _)| *id)?
        };
        self.extract_status(id).ok()
    }
}
