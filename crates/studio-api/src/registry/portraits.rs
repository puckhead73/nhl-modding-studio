//! The supplementary portrait library for historical people: locating the
//! `graphical database/portraits` tree, opening its operator-built SQLite index (or, for a tree
//! shipped without one, indexing the filenames), matching a corpus person against it under one of
//! the four identity tiers, free-text search, image delivery, and the per-person selection sidecar
//! (`historical_portraits.json` beside the corpus).
//!
//! Selections are stored by the portrait's index-relative path, keyed by corpus `person_id`, so a
//! rebuilt data pack keeps them as long as ids are stable, and a moved library keeps them as long as
//! the tree layout is unchanged. The Roster Builder import consumes [`Registry::selected_portrait_path`]
//! when it later writes a legend's portrait into the target title.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use studio_domain::portrait_index::PortraitIndex;
use studio_domain::portrait_library::{
    LibraryStats, MatchMode, PersonKey, PortraitEntry, PortraitLibrary,
};

use super::Registry;
use crate::dto::{HistoricalPortraitMatchesDto, PortraitCandidateDto, PortraitLibraryStatusDto};
use crate::error::ApiError;

/// The sidecar file: an optional explicit root plus the per-person selections.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct PortraitSidecar {
    #[serde(default)]
    pub(super) root: Option<String>,
    /// person_id → index-relative portrait path.
    #[serde(default)]
    selections: BTreeMap<i64, String>,
}

const SIDECAR: &str = "historical_portraits.json";
/// The folder name the library ships under, beside the `corpus/` folder.
const LIBRARY_DIR: &[&str] = &["graphical database", "portraits"];

/// The two ways a library root can be read. The SQLite index is authoritative when present: it
/// carries the resolved name boundaries, provenance and the FTS5 search table. The scan exists for
/// a tree copied without its index.
pub enum Backend {
    Index(PortraitIndex),
    Scan(PortraitLibrary),
}

impl Backend {
    fn open(root: &Path) -> Result<Self, ApiError> {
        if PortraitIndex::exists_in(root) {
            return PortraitIndex::open(root)
                .map(Backend::Index)
                .map_err(ApiError::Workspace);
        }
        if root.is_dir() {
            PortraitLibrary::load(root)
                .map(Backend::Scan)
                .map_err(|e| ApiError::Workspace(format!("scan {}: {e}", root.display())))
        } else {
            Ok(Backend::Scan(PortraitLibrary::empty(root)))
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Backend::Index(_) => "sqlite_index",
            Backend::Scan(_) => "filename_scan",
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Backend::Index(_) => false,
            Backend::Scan(l) => l.is_empty(),
        }
    }

    fn stats(&self) -> Result<LibraryStats, ApiError> {
        match self {
            Backend::Index(i) => i.stats().map_err(ApiError::Workspace),
            Backend::Scan(l) => Ok(l.stats()),
        }
    }

    fn library_version(&self) -> Option<String> {
        match self {
            Backend::Index(i) => i.library_version().map(str::to_string),
            Backend::Scan(_) => None,
        }
    }

    fn get(&self, rel_path: &str) -> Option<PortraitEntry> {
        match self {
            Backend::Index(i) => i.get(rel_path),
            Backend::Scan(l) => l.get(rel_path).cloned(),
        }
    }

    fn abs_path(&self, rel_path: &str) -> Option<PathBuf> {
        match self {
            Backend::Index(i) => i.abs_path(rel_path),
            Backend::Scan(l) => l.abs_path(rel_path),
        }
    }

    /// `(page, total)` for a person under one tier.
    fn candidates(
        &self,
        key: &PersonKey,
        mode: MatchMode,
        limit: usize,
    ) -> Result<(Vec<PortraitEntry>, usize), ApiError> {
        match self {
            Backend::Index(i) => i.candidates(key, mode, limit).map_err(ApiError::Workspace),
            Backend::Scan(l) => {
                let all = l.candidates(key, mode);
                let total = all.len();
                Ok((all.into_iter().take(limit).cloned().collect(), total))
            }
        }
    }

    fn search(
        &self,
        query: &str,
        dob: Option<&str>,
        limit: usize,
    ) -> Result<(Vec<PortraitEntry>, usize), ApiError> {
        match self {
            Backend::Index(i) => i.search(query, dob, limit).map_err(ApiError::Workspace),
            Backend::Scan(l) => {
                let page: Vec<PortraitEntry> =
                    l.search(query, dob, limit).into_iter().cloned().collect();
                let total = page.len();
                Ok((page, total))
            }
        }
    }
}

/// Mutable library state held by the registry.
#[derive(Default)]
pub struct PortraitState {
    library: Option<Arc<Backend>>,
}

impl PortraitState {
    /// Drop the cached backend (a new corpus or a changed root).
    pub fn reset(&mut self) {
        self.library = None;
    }
}

impl Registry {
    // ---- locating and loading --------------------------------------------------------------

    fn portrait_sidecar_path(&self) -> Result<PathBuf, ApiError> {
        let dir = self
            .corpus_dir
            .lock()
            .unwrap()
            .clone()
            .ok_or(ApiError::CorpusNotOpen)?;
        Ok(dir.join(SIDECAR))
    }

    pub(super) fn read_portrait_sidecar(&self) -> PortraitSidecar {
        self.portrait_sidecar_path()
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub(super) fn write_portrait_sidecar(&self, sidecar: &PortraitSidecar) -> Result<(), ApiError> {
        let path = self.portrait_sidecar_path()?;
        let json = serde_json::to_string_pretty(sidecar)
            .map_err(|e| ApiError::Workspace(format!("encode {SIDECAR}: {e}")))?;
        std::fs::write(&path, json)
            .map_err(|e| ApiError::Workspace(format!("write {}: {e}", path.display())))
    }

    /// Candidate roots in priority order: the operator's explicit root, then the shipped layout
    /// beside the corpus folder (`<corpus>/../graphical database/portraits`), then inside it.
    fn portrait_root_candidates(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let sidecar = self.read_portrait_sidecar();
        if let Some(r) = sidecar.root.filter(|r| !r.trim().is_empty()) {
            out.push(PathBuf::from(r.trim()));
        }
        if let Some(dir) = self.corpus_dir.lock().unwrap().clone() {
            let shipped = |base: &Path| {
                LIBRARY_DIR
                    .iter()
                    .fold(base.to_path_buf(), |p, s| p.join(s))
            };
            if let Some(parent) = dir.parent() {
                out.push(shipped(parent));
            }
            out.push(shipped(&dir));
        }
        out
    }

    /// The library backend, opened on first use. `Err` only when no corpus is open or the index
    /// is unreadable; an absent tree yields an empty scan whose status says so.
    fn portrait_library(&self) -> Result<Arc<Backend>, ApiError> {
        if let Some(lib) = self.portraits.lock().unwrap().library.clone() {
            return Ok(lib);
        }
        if self.corpus_dir.lock().unwrap().is_none() {
            return Err(ApiError::CorpusNotOpen);
        }
        let candidates = self.portrait_root_candidates();
        let root = candidates
            .iter()
            .find(|p| p.is_dir())
            .cloned()
            .or_else(|| candidates.first().cloned())
            .unwrap_or_else(|| PathBuf::from("graphical database/portraits"));
        let arc = Arc::new(Backend::open(&root)?);
        self.portraits.lock().unwrap().library = Some(arc.clone());
        Ok(arc)
    }

    // ---- public surface ----------------------------------------------------------------------

    /// Library status: where it was found, which backend reads it, how much it holds.
    pub fn portrait_library_status(&self) -> Result<PortraitLibraryStatusDto, ApiError> {
        let lib = self.portrait_library()?;
        let stats = lib.stats()?;
        let selections = self.read_portrait_sidecar().selections.len();
        Ok(PortraitLibraryStatusDto {
            root: stats.root,
            available: !lib.is_empty(),
            backend: lib.kind().to_string(),
            library_version: lib.library_version(),
            portrait_count: stats.portrait_count,
            person_count: stats.person_count,
            csv_path: stats.csv_path,
            csv_rows: stats.csv_rows,
            selection_count: selections,
            warnings: stats.warnings,
        })
    }

    /// Point the library at another folder (empty clears the override) and reopen.
    pub fn portrait_library_set_root(
        &self,
        root: &str,
    ) -> Result<PortraitLibraryStatusDto, ApiError> {
        let mut sidecar = self.read_portrait_sidecar();
        sidecar.root = Some(root.trim().to_string()).filter(|r| !r.is_empty());
        self.write_portrait_sidecar(&sidecar)?;
        self.portraits.lock().unwrap().reset();
        self.portrait_library_status()
    }

    /// Reopen the current root (after the operator rebuilt the index or added files).
    pub fn portrait_library_rescan(&self) -> Result<PortraitLibraryStatusDto, ApiError> {
        self.portraits.lock().unwrap().reset();
        self.portrait_library_status()
    }

    /// A corpus person's matching portraits under `mode`, plus their current selection.
    pub fn historical_portrait_candidates(
        &self,
        person_id: i64,
        mode: &str,
        limit: usize,
    ) -> Result<HistoricalPortraitMatchesDto, ApiError> {
        let mode = MatchMode::parse(mode).ok_or_else(|| {
            ApiError::Workspace(format!(
                "unknown portrait match mode {mode:?} (exact, last_dob, last_only, dob_only)"
            ))
        })?;
        let key = self.with_corpus(|c| {
            let d = c
                .person_detail(person_id)?
                .ok_or_else(|| ApiError::Workspace(format!("no historical person #{person_id}")))?;
            Ok(PersonKey {
                last_name: d.last_name,
                first_name: d.first_name,
                dob: Some(d.dob).filter(|s| s != "?"),
            })
        })?;
        let lib = self.portrait_library()?;
        let (page, total) = lib.candidates(&key, mode, limit)?;
        Ok(HistoricalPortraitMatchesDto {
            mode: mode.as_str().to_string(),
            total,
            candidates: page.iter().map(|e| candidate_dto(e, Some(mode))).collect(),
            selected: self.selected_portrait(person_id, &lib),
            library_available: !lib.is_empty(),
        })
    }

    /// Free-text lookup across recorded names, aliases, slugs (and, for the scan, sources), with
    /// an optional DOB filter.
    pub fn portrait_library_search(
        &self,
        query: &str,
        dob: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PortraitCandidateDto>, ApiError> {
        let lib = self.portrait_library()?;
        let (page, _total) = lib.search(query, dob, limit)?;
        Ok(page.iter().map(|e| candidate_dto(e, None)).collect())
    }

    /// One library image as a PNG data URL, downscaled to fit `max_size` when given (grid
    /// thumbnails); refuses any path that is not an indexed entry.
    pub fn portrait_library_image(
        &self,
        rel_path: &str,
        max_size: Option<u32>,
    ) -> Result<String, ApiError> {
        let lib = self.portrait_library()?;
        let abs = lib.abs_path(rel_path).ok_or_else(|| {
            ApiError::Workspace(format!("portrait {rel_path:?} is not in the library"))
        })?;
        let bytes = std::fs::read(&abs)
            .map_err(|e| ApiError::Workspace(format!("read {}: {e}", abs.display())))?;
        let Some(max) = max_size.filter(|m| *m > 0) else {
            return Ok(format!("data:image/png;base64,{}", BASE64.encode(&bytes)));
        };
        let img = image::load_from_memory(&bytes)
            .map_err(|e| ApiError::Workspace(format!("decode {}: {e}", abs.display())))?;
        let (w, h) = (img.width(), img.height());
        let out = if w <= max && h <= max {
            bytes
        } else {
            let scaled = img.resize(max, max, image::imageops::FilterType::Triangle);
            let mut buf = std::io::Cursor::new(Vec::new());
            scaled
                .write_to(&mut buf, image::ImageFormat::Png)
                .map_err(|e| ApiError::Workspace(format!("encode thumbnail: {e}")))?;
            buf.into_inner()
        };
        Ok(format!("data:image/png;base64,{}", BASE64.encode(&out)))
    }

    /// Record (or clear, with `None`) the portrait chosen for a corpus person.
    pub fn set_historical_portrait(
        &self,
        person_id: i64,
        rel_path: Option<&str>,
    ) -> Result<Option<PortraitCandidateDto>, ApiError> {
        let lib = self.portrait_library()?;
        let mut sidecar = self.read_portrait_sidecar();
        match rel_path.map(str::trim).filter(|p| !p.is_empty()) {
            Some(p) => {
                if lib.get(p).is_none() {
                    return Err(ApiError::Workspace(format!(
                        "portrait {p:?} is not in the library"
                    )));
                }
                sidecar.selections.insert(person_id, p.to_string());
            }
            None => {
                sidecar.selections.remove(&person_id);
            }
        }
        self.write_portrait_sidecar(&sidecar)?;
        Ok(self.selected_portrait(person_id, &lib))
    }

    /// The selected portrait for a person, if any (and still present in the library).
    pub fn historical_portrait_selected(
        &self,
        person_id: i64,
    ) -> Result<Option<PortraitCandidateDto>, ApiError> {
        let lib = self.portrait_library()?;
        Ok(self.selected_portrait(person_id, &lib))
    }

    /// Absolute path of a person's selected portrait — the hook the Roster Builder import uses.
    pub fn selected_portrait_path(&self, person_id: i64) -> Option<PathBuf> {
        let lib = self.portrait_library().ok()?;
        let rel = self.read_portrait_sidecar().selections.remove(&person_id)?;
        lib.abs_path(&rel)
    }

    fn selected_portrait(&self, person_id: i64, lib: &Backend) -> Option<PortraitCandidateDto> {
        let rel = self.read_portrait_sidecar().selections.remove(&person_id)?;
        lib.get(&rel).map(|e| candidate_dto(&e, None))
    }
}

fn candidate_dto(e: &PortraitEntry, tier: Option<MatchMode>) -> PortraitCandidateDto {
    PortraitCandidateDto {
        rel_path: e.rel_path.clone(),
        stem: e.stem.clone(),
        display_name: e.display_name(),
        first_name: e.first_name.clone(),
        last_name: e.last_name.clone(),
        dob: e.dob.clone(),
        variant: e.variant,
        source: e.source.clone(),
        source_type: e.source_type.clone(),
        games: e.games.clone(),
        identity_status: e.identity_status.clone(),
        tier: tier.map(|t| t.as_str().to_string()),
    }
}
