//! The loose-file texture Replace route as a **reviewed operation** (coordinator review of
//! 24 Sep 2026, F1). A plan is a server-held review: the target's bytes (hash and length), the
//! resolved target path, the raster index, the source's identity (a file's path and hash, or the
//! UI-baked PNG itself) and dimensions, and the policy the operator was shown (container, checked,
//! allowed, refusal, notes). A write names the review it applies; at the mutation boundary it reads
//! the target and the source **once**, compares them and the recomputed policy against the review,
//! refuses any drift, and hands those very bytes to the writer. Nothing is re-planned behind the
//! review: a materially different operation needs a freshly displayed plan.
//!
//! Why the policy is compared and not just the bytes: the 24 Sep defect was an allowed-to-allowed
//! change — a 512² single-level raster (full replacement, no notes) replaced at the same path by a
//! 10-level raster (a partial replacement keeping seven packed-tail levels). Rechecking whether the
//! write is *allowed* would still have passed; the operator would have reviewed a different
//! operation than the one performed. The byte hash catches that case; the policy comparison
//! states the contract and guards any future path where equal bytes could plan differently
//! (a texture-index or dimension change reaching the writer under an old review).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use sha2::{Digest, Sha256};
use studio_domain::DecodedImage;

use super::Registry;
use crate::dto::{ImportTextureSummaryDto, TextureReplacePlanDto};
use crate::error::ApiError;

/// Reviews held at once. A bytes-route review holds its decoded source image; the bound keeps the
/// held memory small while leaving room for a few open dialogs.
const MAX_HELD_REVIEWS: usize = 8;

/// The source the operator reviewed: a file (re-read and re-hashed at write time) or the PNG the UI
/// baked (held here — the write cannot receive a different image than the one reviewed).
pub(super) enum ReviewedSource {
    File { path: PathBuf, sha256: [u8; 32] },
    Bytes { image: DecodedImage },
}

/// The policy as displayed. Compared field by field at write time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReviewedPolicy {
    container: Option<String>,
    checked: bool,
    allowed: bool,
    refusal: Option<String>,
    notes: Vec<String>,
}

pub(super) struct TextureReview {
    /// The install root exactly as the caller named it (the project / tree selection).
    install_dir: String,
    rel_path: String,
    texture_index: Option<u32>,
    target: PathBuf,
    target_len: u64,
    target_sha256: [u8; 32],
    source: ReviewedSource,
    source_width: u32,
    source_height: u32,
    policy: ReviewedPolicy,
}

#[derive(Default)]
pub(super) struct TextureReviewState {
    reviews: HashMap<u64, TextureReview>,
    /// Insertion order, oldest first, for the eviction bound.
    order: Vec<u64>,
    next_review: u64,
}

impl TextureReviewState {
    fn insert(&mut self, review: TextureReview) -> u64 {
        self.next_review += 1;
        let id = self.next_review;
        self.reviews.insert(id, review);
        self.order.push(id);
        while self.order.len() > MAX_HELD_REVIEWS {
            let old = self.order.remove(0);
            self.reviews.remove(&old);
        }
        id
    }
    fn take(&mut self, id: u64) -> Option<TextureReview> {
        self.order.retain(|x| *x != id);
        self.reviews.remove(&id)
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

#[allow(dead_code)]
fn resolve_target(install_dir: &str, rel_path: &str) -> Result<PathBuf, ApiError> {
    studio_domain::resolve_install_path(Path::new(install_dir), rel_path)
        .ok_or_else(|| ApiError::Workspace(format!("{rel_path} escapes the install root")))
}

fn read_target(target: &Path) -> Result<Vec<u8>, ApiError> {
    std::fs::read(target)
        .map_err(|e| ApiError::Workspace(format!("could not read {}: {e}", target.to_string_lossy())))
}

fn policy_of(plan: &studio_domain::TextureReplacePlan) -> ReviewedPolicy {
    ReviewedPolicy {
        container: plan.container.map(|c| c.label().to_string()),
        checked: plan.checked,
        allowed: plan.allowed,
        refusal: plan.refusal.clone(),
        notes: plan.notes.clone(),
    }
}

fn plan_dto(review_id: u64, policy: &ReviewedPolicy, w: u32, h: u32) -> TextureReplacePlanDto {
    TextureReplacePlanDto {
        review_id,
        container: policy.container.clone(),
        checked: policy.checked,
        allowed: policy.allowed,
        refusal: policy.refusal.clone(),
        notes: policy.notes.clone(),
        source_width: w,
        source_height: h,
    }
}

/// A refusal at the mutation boundary. Every message ends with the same instruction so the dialog
/// can say what to do; nothing has been backed up or written when one is returned.
fn drift(what: impl std::fmt::Display) -> ApiError {
    ApiError::Workspace(format!(
        "The reviewed write was not performed: {what}. Nothing was written and no backup was made; review the replacement again before writing."
    ))
}

impl Registry {
    /// What writing the image at `source_image_path` into the texture at `rel_path` would do —
    /// the pre-Apply twin of [`Registry::import_texture`]'s refusal and disclosures, computed from
    /// the target's bytes without encoding or writing anything. The answer is held as a review
    /// (`review_id`) that the write must name; the review binds the target's bytes, the raster,
    /// the source file's bytes and dimensions and the displayed policy.
    pub fn texture_replace_plan(
        &self,
        install_dir: &str,
        rel_path: &str,
        source_image_path: &str,
        texture_index: Option<u32>,
    ) -> Result<TextureReplacePlanDto, ApiError> {
        let source_path = PathBuf::from(source_image_path);
        let source_bytes = std::fs::read(&source_path)
            .map_err(|e| ApiError::Workspace(format!("could not read the source image {source_image_path}: {e}")))?;
        let img = studio_domain::decode_source_image_bytes(&source_bytes)
            .map_err(|e| ApiError::Domain(studio_domain::DomainError::Extract(format!("{source_image_path}: {e}"))))?;
        let source = ReviewedSource::File { path: source_path, sha256: sha256(&source_bytes) };
        self.review_texture_replace(install_dir, rel_path, texture_index, source, img.width, img.height)
    }

    /// [`Registry::texture_replace_plan`] for a PNG the UI baked (the nameplate editor). The
    /// decoded image is held in the review, so [`Registry::import_texture_bytes`] writes exactly
    /// the reviewed image.
    pub fn texture_replace_plan_bytes(
        &self,
        install_dir: &str,
        rel_path: &str,
        png_base64: &str,
        texture_index: Option<u32>,
    ) -> Result<TextureReplacePlanDto, ApiError> {
        let png = BASE64
            .decode(png_base64)
            .map_err(|e| ApiError::Workspace(format!("invalid base64 PNG: {e}")))?;
        let img = studio_domain::decode_source_image_bytes(&png).map_err(ApiError::Domain)?;
        let (w, h) = (img.width, img.height);
        self.review_texture_replace(install_dir, rel_path, texture_index, ReviewedSource::Bytes { image: img }, w, h)
    }

    fn review_texture_replace(
        &self,
        install_dir: &str,
        rel_path: &str,
        texture_index: Option<u32>,
        source: ReviewedSource,
        source_width: u32,
        source_height: u32,
    ) -> Result<TextureReplacePlanDto, ApiError> {
        // ONB-002: inside a project the target is the project's overlay copy of the installed
        // file (materialised on first use); the installation itself is never the write target.
        let (target, _) = self.project_asset_target(install_dir, rel_path)?;
        let file_bytes = read_target(&target)?;
        let plan = studio_domain::plan_texture_replace(&file_bytes, source_width, source_height, texture_index.map(|i| i as usize))
            .map_err(ApiError::Domain)?;
        let policy = policy_of(&plan);
        let review = TextureReview {
            install_dir: install_dir.to_string(),
            rel_path: rel_path.to_string(),
            texture_index,
            target,
            target_len: file_bytes.len() as u64,
            target_sha256: sha256(&file_bytes),
            source,
            source_width,
            source_height,
            policy: policy.clone(),
        };
        let id = self.texture_reviews.lock().unwrap().insert(review);
        Ok(plan_dto(id, &policy, source_width, source_height))
    }

    /// Write the reviewed source file into the extracted asset file at `rel_path` (resolved
    /// **safely under `install_dir`**), dispatching on the file's container. `review_id` names the
    /// plan the operator saw; the write refuses — before any backup or mutation — when the target's
    /// bytes, the source file's bytes, the raster, the selection or the recomputed policy differ
    /// from that review. On `write_backup`, a one-time `<target>.bak` is written before the first
    /// overwrite (kept if it already exists). A review is consumed by the write that names it,
    /// whatever the outcome; a refused or repeated write needs a fresh plan.
    pub fn import_texture(
        &self,
        install_dir: &str,
        rel_path: &str,
        source_image_path: &str,
        write_backup: bool,
        texture_index: Option<u32>,
        review_id: u64,
    ) -> Result<ImportTextureSummaryDto, ApiError> {
        let review = self.take_review(review_id)?;
        let (resolved, project) = self.project_asset_target(install_dir, rel_path)?;
        let (file_bytes, target) = check_target(&review, install_dir, rel_path, texture_index, &resolved)?;
        let write_backup = write_backup && project.is_none();
        let img = match &review.source {
            ReviewedSource::File { path, sha256: reviewed } => {
                if Path::new(source_image_path) != path.as_path() {
                    return Err(drift(format!(
                        "the source image is not the reviewed one ({} was reviewed, {source_image_path} was given)",
                        path.display()
                    )));
                }
                let source_bytes = std::fs::read(path)
                    .map_err(|e| drift(format!("the reviewed source image {} could not be read ({e})", path.display())))?;
                if sha256(&source_bytes) != *reviewed {
                    return Err(drift(format!("the source image {} changed since it was reviewed", path.display())));
                }
                studio_domain::decode_source_image_bytes(&source_bytes)
                    .map_err(|e| drift(format!("the reviewed source image no longer decodes ({e})")))?
            }
            ReviewedSource::Bytes { .. } => {
                return Err(drift("this review holds a baked image, not a source file; write it through the bytes route"));
            }
        };
        let out = write_reviewed(&review, &target, file_bytes, &img, write_backup)?;
        if let Some(id) = project {
            // ONB-003: the source image is kept with the project (content-addressed) so the edit
            // stays portable intent.
            let src = Path::new(source_image_path);
            let bytes = std::fs::read(src).map_err(|e| ApiError::Workspace(format!("the source image {} could not be re-read for the project record: {e}", src.display())))?;
            let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("bin");
            let name = src.file_name().and_then(|n| n.to_str());
            self.record_staged_output(id, install_dir, rel_path, "texture-replace", Some((&bytes, ext, name)), texture_index)?;
        }
        Ok(out)
    }

    /// Write the image held by review `review_id` (a PNG the UI baked, reviewed through
    /// [`Registry::texture_replace_plan_bytes`]) into the texture at `rel_path`. Same safe-join,
    /// drift refusals, container dispatch and one-time `.bak` as [`Registry::import_texture`].
    pub fn import_texture_bytes(
        &self,
        install_dir: &str,
        rel_path: &str,
        write_backup: bool,
        texture_index: Option<u32>,
        review_id: u64,
    ) -> Result<ImportTextureSummaryDto, ApiError> {
        let review = self.take_review(review_id)?;
        let (resolved, project) = self.project_asset_target(install_dir, rel_path)?;
        let (file_bytes, target) = check_target(&review, install_dir, rel_path, texture_index, &resolved)?;
        let write_backup = write_backup && project.is_none();
        let img = match &review.source {
            ReviewedSource::Bytes { image } => image,
            ReviewedSource::File { path, .. } => {
                return Err(drift(format!(
                    "this review is for the source file {}; write it through the file route",
                    path.display()
                )));
            }
        };
        let out = write_reviewed(&review, &target, file_bytes, img, write_backup)?;
        if let Some(id) = project {
            // ONB-003: the baked image is the editable source of this output.
            let png = img.to_png().map_err(ApiError::Domain)?;
            self.record_staged_output(id, install_dir, rel_path, "nameplate", Some((&png, "png", None)), texture_index)?;
        }
        Ok(out)
    }

    fn take_review(&self, review_id: u64) -> Result<TextureReview, ApiError> {
        self.texture_reviews.lock().unwrap().take(review_id).ok_or_else(|| {
            drift(format!(
                "review {review_id} is not current (it was already used for a write, superseded, or never existed)"
            ))
        })
    }
}

/// The target side of the review: same selection (install root, path, raster), same resolved
/// path, and the bytes on disk — read **once** here and returned for the writer — match the
/// reviewed hash and length.
fn check_target(
    review: &TextureReview,
    install_dir: &str,
    rel_path: &str,
    texture_index: Option<u32>,
    resolved: &Path,
) -> Result<(Vec<u8>, PathBuf), ApiError> {
    if review.install_dir != install_dir {
        return Err(drift(format!(
            "the review was made for the install root {} but the write names {install_dir}",
            review.install_dir
        )));
    }
    if review.rel_path != rel_path {
        return Err(drift(format!("the review was made for {} but the write names {rel_path}", review.rel_path)));
    }
    if review.texture_index != texture_index {
        return Err(drift(format!(
            "the review was made for raster {} but the write names raster {}",
            fmt_idx(review.texture_index),
            fmt_idx(texture_index)
        )));
    }
    let target = resolved.to_path_buf();
    if target != review.target {
        return Err(drift(format!(
            "the target resolves to {} but the review was made for {}",
            target.display(),
            review.target.display()
        )));
    }
    let file_bytes = read_target(&target)?;
    if file_bytes.len() as u64 != review.target_len || sha256(&file_bytes) != review.target_sha256 {
        return Err(drift(format!(
            "the texture file {} changed since the plan was shown (the reviewed bytes are not the bytes on disk)",
            target.display()
        )));
    }
    Ok((file_bytes, target))
}

fn fmt_idx(i: Option<u32>) -> String {
    i.map(|i| i.to_string()).unwrap_or_else(|| "(primary)".to_string())
}

/// The reviewed policy recomputed from the very bytes the writer consumes, then the write. The
/// policy is a function of the target bytes, the source dimensions and the raster, all of which
/// were just checked, so a difference here means the plan and the writer disagree — refuse rather
/// than perform an operation the operator did not see.
fn write_reviewed(
    review: &TextureReview,
    target: &Path,
    file_bytes: Vec<u8>,
    img: &DecodedImage,
    write_backup: bool,
) -> Result<ImportTextureSummaryDto, ApiError> {
    if (img.width, img.height) != (review.source_width, review.source_height) {
        return Err(drift(format!(
            "the source image is {}×{} but {}×{} was reviewed",
            img.width, img.height, review.source_width, review.source_height
        )));
    }
    let now = studio_domain::plan_texture_replace(&file_bytes, img.width, img.height, review.texture_index.map(|i| i as usize))
        .map_err(ApiError::Domain)?;
    let now = policy_of(&now);
    if now != review.policy {
        return Err(drift(format!(
            "what this write would do differs from the plan that was shown (shown: {}; now: {})",
            describe(&review.policy),
            describe(&now)
        )));
    }
    if !now.allowed {
        // The review itself was a refusal: the dialog never enables Write on one, and the writer
        // below would refuse too, but the contract is stated here rather than relied on.
        return Err(ApiError::Workspace(now.refusal.clone().unwrap_or_else(|| "the reviewed plan refused this write".into())));
    }
    let target_str = target.to_string_lossy().into_owned();
    let write = studio_domain::replace_texture_in_file(
        file_bytes,
        &img.rgba,
        img.width,
        img.height,
        review.texture_index.map(|i| i as usize),
    )
    .map_err(ApiError::Domain)?;
    if write.notes != review.policy.notes {
        // The writer's own disclosure must be the one reviewed. This cannot differ when the plan
        // above matched, but the promise "the result repeats the plan word for word" is enforced
        // before any mutation, not assumed.
        return Err(drift(format!(
            "the writer's disclosure differs from the reviewed plan (shown: {:?}; writer: {:?})",
            review.policy.notes, write.notes
        )));
    }

    let backup_path = if write_backup {
        let bak = format!("{target_str}.bak");
        if !Path::new(&bak).exists() {
            std::fs::copy(target, &bak)
                .map_err(|e| ApiError::Workspace(format!("could not write backup {bak}: {e}")))?;
        }
        Some(bak)
    } else {
        None
    };
    std::fs::write(target, &write.bytes)
        .map_err(|e| ApiError::Workspace(format!("could not write {target_str}: {e}")))?;

    Ok(ImportTextureSummaryDto {
        target_path: target_str,
        container: write.container.label().to_string(),
        width: write.width,
        height: write.height,
        bytes_written: write.bytes.len() as u64,
        backup_path,
        notes: write.notes.clone(),
    })
}

fn describe(p: &ReviewedPolicy) -> String {
    let container = p.container.as_deref().unwrap_or("unrecognised container");
    match (&p.checked, &p.allowed, &p.refusal) {
        (false, _, _) => format!("{container}, no pre-check"),
        (true, false, Some(r)) => format!("{container}, refused: {r}"),
        (true, false, None) => format!("{container}, refused"),
        (true, true, _) if p.notes.is_empty() => format!("{container}, full replacement"),
        (true, true, _) => format!("{container}, {} disclosure note(s): {}", p.notes.len(), p.notes.join(" | ")),
    }
}
