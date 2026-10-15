//! Filesystem boundary enforcement for every evidence mutation entry point.
//!
//! Added by **C0-BULK1-R1** after review. The first implementation checked output locations only in
//! the CLI, so a library caller — or a path that *aliased* a protected location through `..`, a
//! symlink or a Windows junction — could write into the evidence tree, the manual records, the
//! frozen legacy sources or the published set. The checks now live here and are called from the
//! mutating functions themselves, so no caller can skip them.
//!
//! Two operations, both of which must succeed **before any byte is written or deleted**:
//!
//! | Function | Guarantee |
//! |---|---|
//! | [`resolve_for_write`] | the path contains no `..`/`.` traversal, every **existing** ancestor is a real directory and not a link or reparse point, and the result is the OS-canonical form of the deepest existing ancestor plus the literal remainder |
//! | [`assert_disjoint`] | two resolved locations are neither equal nor nested in either direction |
//!
//! Resolution is what makes alias rejection work: `canonicalize` follows junctions and symlinks, so
//! two names for one directory resolve to the same path and [`assert_disjoint`] rejects the pair.
//! Rejecting reparse points outright is the belt to that braces — a link *inside* the resolved
//! prefix could otherwise be swapped between the check and the write.
//!
//! **C0-BULK1-R2.** The R1 implementation resolved the *directory* an output went into and then
//! wrote its children with a bare `fs::write`, which follows an existing child symlink. Every
//! mutation target is now validated in its own right by [`resolve_file_for_write`], and
//! [`write_file`] / [`copy_file`] / [`rename_file`] are the only write paths the evidence code uses.
//! The link test is the Windows reparse-point **attribute**, not `is_symlink` alone, and presence is
//! established with `symlink_metadata` so a **dangling** link is seen as a link rather than as an
//! absent file.
//!
//! R2 also removes the optional-protection bypass: [`Layout`] is a *validated* evidence layout that
//! every production mutation entry point now requires. There is no public wrapper left that passes
//! an empty protection set, so a library caller receives exactly the checks the CLI does.
//!
//! **What this does not defend against.** These are check-then-act guards, not locks. A sufficiently
//! privileged concurrent writer can still replace a component after the check and before the write
//! (a TOCTOU race). Nothing here claims otherwise, and no evidence operation takes a filesystem
//! lock; the no-concurrent-writer limitation is documented, not removed.

use std::fs;
use std::path::{Component, Path, PathBuf};

use super::store::StoreError;

type Result<T> = std::result::Result<T, StoreError>;

fn rule(name: &'static str, path: &Path, detail: impl Into<String>) -> StoreError {
    StoreError::Rule {
        rule: name,
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

/// Resolve a path that is about to be written to.
///
/// Returns the canonical form of the deepest **existing** ancestor with the not-yet-existing
/// remainder appended. Rejects, before returning:
///
/// * an empty path;
/// * any `..` component anywhere — traversal is refused rather than normalised away, because
///   normalising it silently is exactly how an output path reaches a protected root. A `.` is also
///   refused, though only a **leading** one can reach this check: Rust's own `Path::components`
///   drops interior `.` components before we see them, and an interior `.` cannot escape a
///   directory in any case;
/// * an existing ancestor that is a symlink, junction or other reparse point;
/// * an existing ancestor that exists but is not a directory.
///
/// Callers must compare **resolved** paths with each other; comparing a resolved path against a raw
/// one is meaningless on Windows, where the canonical form carries a `\\?\` prefix.
pub fn resolve_for_write(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(rule("path-empty", path, "an output path may not be empty"));
    }
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err(rule(
                    "path-traversal",
                    path,
                    "`..` is refused in an output path rather than normalised away",
                ));
            }
            Component::CurDir => {
                return Err(rule(
                    "path-traversal",
                    path,
                    "`.` is refused in an output path",
                ));
            }
            _ => {}
        }
    }

    // Deepest existing ancestor, and the tail below it.
    let mut existing = path.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name().map(ToOwned::to_owned) else {
            return Err(rule(
                "path-unrooted",
                path,
                "no ancestor of this path exists; create the parent explicitly first",
            ));
        };
        tail.push(name);
        if !existing.pop() {
            return Err(rule("path-unrooted", path, "no existing ancestor"));
        }
    }
    tail.reverse();

    assert_no_reparse_points(&existing)?;
    let metadata = fs::symlink_metadata(&existing).map_err(|source| StoreError::Io {
        path: existing.clone(),
        source,
    })?;
    if !metadata.is_dir() {
        // The deepest existing ancestor is the file itself only when the whole path exists; that is
        // legitimate (an overwrite target), so only a non-directory *ancestor* is an error.
        if !tail.is_empty() {
            return Err(rule(
                "path-not-a-directory",
                &existing,
                "an ancestor of the output path exists and is not a directory",
            ));
        }
    }

    let mut resolved = existing.canonicalize().map_err(|source| StoreError::Io {
        path: existing.clone(),
        source,
    })?;
    for part in tail {
        resolved.push(part);
    }
    Ok(resolved)
}

/// Is this entry a symlink, junction or any other reparse point?
///
/// **R2:** on Windows `FileType::is_symlink` covers symbolic links and mount points but not every
/// reparse class, so `FILE_ATTRIBUTE_REPARSE_POINT` is tested directly as well. Elsewhere
/// `is_symlink` is the whole story.
pub fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Reject a path any of whose components is a symlink, junction or other reparse point.
///
/// On Windows `FileType::is_symlink` is true for both symbolic links and mount points (junctions),
/// which is exactly the alias class this must refuse; [`is_reparse_point`] additionally tests the
/// reparse-point attribute so no other reparse class slips through.
pub fn assert_no_reparse_points(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        // A prefix (`C:`, `\\?\H:`, a UNC share) and a bare root are not entries that can be
        // stat'ed — on Windows a verbatim prefix returns `ERROR_INVALID_FUNCTION` rather than a
        // metadata record — and neither can be a reparse point. Everything below them is checked.
        if matches!(component, Component::Prefix(_) | Component::RootDir) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if is_reparse_point(&metadata) {
                    return Err(rule(
                        "path-reparse-point",
                        &current,
                        "a link, junction or reparse point on the path to an evidence location is \
                         refused: it can alias a protected root, and it can be re-pointed between \
                         the check and the write",
                    ));
                }
            }
            // A component that does not exist cannot be a link.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(StoreError::Io {
                    path: current.clone(),
                    source,
                });
            }
        }
    }
    Ok(())
}

/// Refuse two locations that are equal or nested either way.
///
/// Both arguments must already have been through [`resolve_for_write`]; the labels appear verbatim
/// in the diagnostic so the message names which two roots collided.
pub fn assert_disjoint(label_a: &str, a: &Path, label_b: &str, b: &Path) -> Result<()> {
    if a == b {
        return Err(rule(
            "path-overlap",
            a,
            format!("{label_a} and {label_b} resolve to the same location"),
        ));
    }
    if a.starts_with(b) {
        return Err(rule(
            "path-overlap",
            a,
            format!("{label_a} resolves inside {label_b}"),
        ));
    }
    if b.starts_with(a) {
        return Err(rule(
            "path-overlap",
            b,
            format!("{label_b} resolves inside {label_a}"),
        ));
    }
    Ok(())
}

/// The protected roots under an evidence root that no generated output may ever alias.
///
/// `manual/` is layer B (R-SEP), `legacy/frozen/` holds the byte-for-byte migration sources
/// (R-FREEZE), and `auto/runs/` holds immutable run directories.
pub fn protected_under(evidence_root: &Path) -> Vec<(&'static str, PathBuf)> {
    vec![
        ("the evidence root", evidence_root.to_path_buf()),
        ("the manual records root", evidence_root.join("manual")),
        (
            "the frozen legacy sources",
            evidence_root.join("legacy").join("frozen"),
        ),
        (
            "the automated run directories",
            evidence_root.join("auto").join("runs"),
        ),
    ]
}

/// Resolve `candidate` and refuse it if it aliases any protected root.
///
/// A protected root that does not exist yet is resolved leniently — it cannot be aliased by anything
/// if it is absent, but its *prefix* is still resolved so a future creation cannot be pre-aliased.
pub fn assert_output_is_safe(
    candidate_label: &str,
    candidate: &Path,
    protected: &[(&str, PathBuf)],
) -> Result<PathBuf> {
    let resolved = resolve_for_write(candidate)?;
    for (label, root) in protected {
        // A protected root whose own path is malformed is a caller error, not a reason to skip the
        // check: fail closed.
        let resolved_root = resolve_for_write(root)?;
        assert_disjoint(candidate_label, &resolved, label, &resolved_root)?;
    }
    Ok(resolved)
}

/// Resolve a path that is about to be **written as a file**, validating the final component too.
///
/// [`resolve_for_write`] proves the *directory* an output lands in is not aliased. That is not
/// enough: `fs::write` follows an existing child symlink, so a planted — or merely mistaken — link
/// named `qa-evidence-matrix.csv` redirects the write outside the resolved directory. This function
/// additionally inspects the final component with `symlink_metadata` (**not** `exists`, which
/// reports a dangling link as absent) and refuses:
///
/// * a link, junction or any other reparse point — dangling or not (`path-reparse-point`);
/// * an existing directory where a file is to be written (`path-unexpected-entry-type`);
/// * any other non-regular entry type (`path-unexpected-entry-type`).
///
/// A final component that does not exist is fine: that is the ordinary create case.
pub fn resolve_file_for_write(path: &Path) -> Result<PathBuf> {
    let resolved = resolve_for_write(path)?;
    match fs::symlink_metadata(&resolved) {
        Ok(metadata) => {
            if is_reparse_point(&metadata) {
                return Err(rule(
                    "path-reparse-point",
                    &resolved,
                    "the mutation target itself is a link, junction or reparse point; writing \
                     through it would place the bytes somewhere else entirely. A dangling link is \
                     refused on the same grounds — `symlink_metadata` sees it even though \
                     `exists()` reports it absent",
                ));
            }
            if !metadata.file_type().is_file() {
                return Err(rule(
                    "path-unexpected-entry-type",
                    &resolved,
                    format!(
                        "the mutation target exists and is not a regular file ({:?})",
                        metadata.file_type()
                    ),
                ));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(StoreError::Io {
                path: resolved,
                source,
            });
        }
    }
    Ok(resolved)
}

/// The write path the evidence code uses: validate the target, then write it.
///
/// Returns the resolved path actually written, so a caller that hashes or moves it afterwards uses
/// the same resolved name rather than re-deriving it.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let resolved = resolve_file_for_write(path)?;
    fs::write(&resolved, bytes).map_err(|source| StoreError::Io {
        path: resolved.clone(),
        source,
    })?;
    Ok(resolved)
}

/// **Append** to the target, validating it as a mutation target first.
///
/// This is the durable half of the emitter journal (`evidence::emit`). A whole-file rewrite loses
/// every previously persisted record when the write is interrupted part-way; an append can only
/// ever damage the record it is writing, and the loader drops that trailing fragment and keeps the
/// rest. Nothing here flushes to the device, so this is durability against **process**
/// interruption, not against power loss — the same boundary the rest of this module draws.
pub fn append_file(path: &Path, bytes: &[u8]) -> Result<PathBuf> {
    use std::io::Write;
    let resolved = resolve_file_for_write(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&resolved)
        .map_err(|source| StoreError::Io {
            path: resolved.clone(),
            source,
        })?;
    file.write_all(bytes).map_err(|source| StoreError::Io {
        path: resolved.clone(),
        source,
    })?;
    file.flush().map_err(|source| StoreError::Io {
        path: resolved.clone(),
        source,
    })?;
    Ok(resolved)
}

/// Copy `from` to `to`, validating the destination as a mutation target first.
pub fn copy_file(from: &Path, to: &Path) -> Result<PathBuf> {
    let resolved = resolve_file_for_write(to)?;
    fs::copy(from, &resolved).map_err(|source| StoreError::Io {
        path: from.to_path_buf(),
        source,
    })?;
    Ok(resolved)
}

/// Rename `from` onto `to`, validating the destination as a mutation target first.
///
/// A rename replaces a link rather than following it, but it also destroys whatever the surrounding
/// logic believed was at that name, so the same check applies.
pub fn rename_file(from: &Path, to: &Path) -> Result<PathBuf> {
    let resolved = resolve_file_for_write(to)?;
    fs::rename(from, &resolved).map_err(|source| StoreError::Io {
        path: from.to_path_buf(),
        source,
    })?;
    Ok(resolved)
}

/// A **validated** evidence layout. Constructing one resolves the evidence root and every protected
/// location beneath it; holding one is the proof that a mutation entry point was told where the
/// protected material lives.
///
/// **R2:** this type exists so no production entry point can be called with "no protection". The R1
/// code kept `write_to` / `publish` / `migrate` wrappers that passed an empty protected set, so a
/// library caller silently received weaker checks than the CLI. Those wrappers are gone; the
/// remaining functions take `&Layout`.
#[derive(Debug, Clone)]
pub struct Layout {
    evidence_root: PathBuf,
    protected: Vec<(&'static str, PathBuf)>,
}

impl Layout {
    /// Resolve and validate an evidence root and everything protected beneath it.
    ///
    /// A protected location that does not exist yet is still resolved through its deepest existing
    /// ancestor, so a candidate output cannot pre-alias a directory that is about to be created.
    pub fn new(evidence_root: &Path) -> Result<Self> {
        let root = resolve_for_write(evidence_root)?;
        let mut protected = Vec::new();
        for (label, path) in protected_under(&root) {
            protected.push((label, resolve_for_write(&path)?));
        }
        Ok(Layout {
            evidence_root: root,
            protected,
        })
    }

    pub fn evidence_root(&self) -> &Path {
        &self.evidence_root
    }

    pub fn protected(&self) -> &[(&'static str, PathBuf)] {
        &self.protected
    }

    /// Resolve a candidate output and refuse it if it aliases any protected location.
    pub fn assert_output_is_safe(&self, label: &str, candidate: &Path) -> Result<PathBuf> {
        let resolved = resolve_for_write(candidate)?;
        for (protected_label, root) in &self.protected {
            assert_disjoint(label, &resolved, protected_label, root)?;
        }
        Ok(resolved)
    }
}

// ------------------------------------------------- discovery (C0-S2-INT1-R4) ------
//
// Every evidence walker shares these primitives, so the platform policy — links refused, failures
// propagated, absence distinguished from unreadability — is stated once and cannot drift between
// layers. Discovery **and** digest calculation both go through them, which is what makes "the
// digest covers exactly what discovery saw" a property rather than a coincidence.

/// What a directory entry is, once links have been excluded.
///
/// There is no `Symlink` variant on purpose: a link is rejected *before* classification, so no
/// caller can ever be handed one and decide for itself whether to follow it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
    /// A device, socket, FIFO or anything else that is neither. Never evidence.
    Other,
}

/// Classify one existing entry **without following links**.
///
/// A metadata failure is an error, never an absence: the entry was listed, so something is there,
/// and treating a failed stat as "not a directory" is exactly how a walker silently loses evidence.
pub fn entry_kind(path: &Path) -> Result<EntryKind> {
    let metadata = fs::symlink_metadata(path).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    entry_kind_of(path, &metadata)
}

/// [`entry_kind`] for metadata already in hand.
pub fn entry_kind_of(path: &Path, metadata: &fs::Metadata) -> Result<EntryKind> {
    if is_reparse_point(metadata) {
        return Err(rule(
            "discovery-reparse-point",
            path,
            "a link, junction or reparse point inside an evidence location is refused: it can              alias material outside the root, it can introduce a traversal cycle, and it can be              re-pointed between discovery and reading",
        ));
    }
    Ok(if metadata.is_dir() {
        EntryKind::Dir
    } else if metadata.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    })
}

/// Classify a root that is *allowed* to be absent.
///
/// `Ok(false)` means it does not exist — the only reading of absence any evidence layer is
/// permitted to make (**R-ABSENT**). An existing root of the wrong type, an unreadable one and a
/// link are all errors: `!path.is_dir()` cannot tell those four apart, and answering "absent" for
/// any of the last three converts evidence into silence.
pub fn optional_dir_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => match entry_kind_of(path, &metadata)? {
            EntryKind::Dir => Ok(true),
            _ => Err(rule(
                "discovery-unexpected-root-type",
                path,
                "this evidence root exists but is not a directory; it is not absent, and reading                  it as absent would silently drop everything it should have contained",
            )),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(StoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// One directory's children, sorted, with every failure surfaced.
///
/// A failed *iteration step* is not the end of the listing — entries after it exist and would never
/// be seen — so it propagates rather than being dropped by `.flatten()`.
pub fn read_dir_sorted(dir: &Path) -> Result<Vec<(PathBuf, EntryKind)>> {
    let entries = fs::read_dir(dir).map_err(|source| StoreError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        out.push((path.clone(), entry_kind(&path)?));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Every regular file under `dir`, depth-first in sorted order. An absent `dir` yields nothing.
///
/// **No link is followed and no cycle is possible**: [`entry_kind`] refuses every reparse point, and
/// a tree of real directories cannot contain itself. `visited` additionally records the canonical
/// path of each directory entered, so if a platform ever did present one twice the walk fails with
/// `discovery-cycle` instead of recursing forever.
///
/// `EntryKind::Other` entries are refused rather than skipped: something that is neither a file nor
/// a directory inside an evidence root is unexplained, and unexplained is not the same as absent.
pub fn walk_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    walk_files_into(dir, &mut visited, &mut out)?;
    Ok(out)
}

fn walk_files_into(
    dir: &Path,
    visited: &mut std::collections::BTreeSet<PathBuf>,
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    if !optional_dir_exists(dir)? {
        return Ok(());
    }
    // R5: a canonicalization failure is not a licence to fall back to the raw path — two spellings
    // of the same directory would then look like two directories and the cycle guard would not
    // fire. The directory was just proved to exist, so a failure here means something changed
    // under the walk.
    let canonical = fs::canonicalize(dir).map_err(|source| StoreError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    if !visited.insert(canonical) {
        return Err(rule(
            "discovery-cycle",
            dir,
            "this directory was already entered during the same walk; the traversal would not              terminate and the digest it feeds would be meaningless",
        ));
    }
    for (path, kind) in read_dir_sorted(dir)? {
        match kind {
            EntryKind::Dir => walk_files_into(&path, visited, out)?,
            EntryKind::File => out.push(path),
            EntryKind::Other => {
                return Err(rule(
                    "discovery-unexpected-entry-type",
                    &path,
                    "neither a regular file nor a directory; an unexplained entry inside an                      evidence root is not silently skipped",
                ))
            }
        }
    }
    Ok(())
}

/// Every immediate subdirectory of `dir`, sorted. An absent `dir` yields none.
///
/// `permitted_files` names the non-directory entries allowed to sit alongside them. Anything else is
/// an error: a file where a record directory was expected is either a mistake or evidence someone
/// rescued by hand, and neither should vanish from discovery.
pub fn child_dirs(dir: &Path, permitted_files: &[&str]) -> Result<Vec<PathBuf>> {
    if !optional_dir_exists(dir)? {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for (path, kind) in read_dir_sorted(dir)? {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match kind {
            EntryKind::Dir => out.push(path),
            EntryKind::File if permitted_files.contains(&name.as_str()) => {}
            _ => {
                return Err(rule(
                    "discovery-unexpected-entry",
                    &path,
                    format!(
                        "{name:?} is not a record directory and is not a permitted auxiliary                          entry; ignoring it could hide evidence"
                    ),
                ))
            }
        }
    }
    Ok(out)
}

/// Read a file that is **allowed to be absent** but is not allowed to be unreadable.
///
/// `Ok(None)` means the file does not exist — the only reading of absence an evidence layer may
/// make. Everything else is an error naming the path:
///
/// | Condition | Result |
/// |---|---|
/// | `NotFound` | `Ok(None)` |
/// | a link, junction or reparse point at the name | `discovery-reparse-point` |
/// | a directory or other non-regular entry at the name | `wrong_type_rule` |
/// | not valid UTF-8 | `unreadable_rule` |
/// | any other I/O failure (permission denied, I/O error, …) | [`StoreError::Io`] |
///
/// The rule tokens are parameters because the layers report in their own vocabulary; the
/// *classification* is shared, which is the point. The type check is `symlink_metadata`, so a link
/// is never followed to decide what is at the name.
pub fn read_optional_file(
    path: &Path,
    wrong_type_rule: &'static str,
    unreadable_rule: &'static str,
) -> Result<Option<String>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => match entry_kind_of(path, &metadata)? {
            EntryKind::File => {}
            EntryKind::Dir => {
                return Err(rule(
                    wrong_type_rule,
                    path,
                    "expected a regular file, found a directory; it is not absent, and reading it                      as absent would silently drop whatever it holds",
                ))
            }
            EntryKind::Other => {
                return Err(rule(
                    wrong_type_rule,
                    path,
                    "expected a regular file, found a non-regular entry; it is not absent, and                      reading it as absent would silently drop whatever it holds",
                ))
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(StoreError::Io {
                path: path.to_path_buf(),
                source,
            })
        }
    }
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        // Between the check above and this read the file can be removed by a concurrent writer.
        // That is the documented TOCTOU limit, and absence is still the honest answer for it.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) if source.kind() == std::io::ErrorKind::InvalidData => Err(rule(
            unreadable_rule,
            path,
            "not valid UTF-8; its records cannot be attributed, and an empty list would assert              that there are none",
        )),
        Err(source) => Err(StoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// A synthetic directory. Never a repository, evidence or game path.
    fn sandbox(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir()
            .join("tdb-qa-paths")
            .join(format!("{name}-{}-{n}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).expect("sandbox");
        dir
    }

    #[test]
    fn traversal_is_refused_rather_than_normalised() {
        let root = sandbox("traversal");
        // `..` survives `Path::components`, so it reaches the check and is refused.
        let err = resolve_for_write(&root.join("..").join("escape")).expect_err("must refuse");
        assert!(err.to_string().contains("path-traversal"), "{err}");
        // Several of them, and one buried deep, are refused just the same.
        let deep = root
            .join("a")
            .join("..")
            .join("..")
            .join("..")
            .join("elsewhere");
        let err = resolve_for_write(&deep).expect_err("must refuse");
        assert!(err.to_string().contains("path-traversal"), "{err}");
        // A leading `.` is the only `.` that reaches the check; interior ones are dropped by
        // `Path::components` itself, and cannot escape a directory anyway.
        let err = resolve_for_write(Path::new("./relative/out")).expect_err("must refuse");
        assert!(err.to_string().contains("path-traversal"), "{err}");
    }

    #[test]
    fn an_empty_path_is_refused() {
        let err = resolve_for_write(Path::new("")).expect_err("must refuse");
        assert!(err.to_string().contains("path-empty"), "{err}");
    }

    #[test]
    fn a_not_yet_existing_output_resolves_through_its_existing_ancestor() {
        let root = sandbox("resolve");
        let resolved = resolve_for_write(&root.join("a").join("b")).expect("resolves");
        assert!(
            resolved.ends_with("a/b") || resolved.ends_with("a\\b"),
            "{resolved:?}"
        );
        assert!(resolved.starts_with(root.canonicalize().expect("canonical")));
    }

    #[test]
    fn a_file_used_as_an_ancestor_is_refused() {
        let root = sandbox("file-ancestor");
        let file = root.join("not-a-dir");
        fs::write(&file, b"x").expect("write");
        let err = resolve_for_write(&file.join("child")).expect_err("must refuse");
        assert!(err.to_string().contains("path-not-a-directory"), "{err}");
    }

    #[test]
    fn nesting_is_refused_in_both_directions() {
        let root = sandbox("nesting");
        let outer = resolve_for_write(&root).expect("resolves");
        let inner = resolve_for_write(&root.join("inner")).expect("resolves");
        assert!(assert_disjoint("out", &outer, "in", &inner).is_err());
        assert!(assert_disjoint("in", &inner, "out", &outer).is_err());
        assert!(assert_disjoint("a", &outer, "b", &outer).is_err());

        let sibling = resolve_for_write(&sandbox("nesting-sibling")).expect("resolves");
        assert!(assert_disjoint("a", &outer, "b", &sibling).is_ok());
    }

    #[test]
    fn an_output_inside_a_protected_root_is_refused() {
        let evidence = sandbox("protected");
        fs::create_dir_all(evidence.join("manual")).expect("dirs");
        let protected = protected_under(&evidence);
        let err = assert_output_is_safe(
            "the candidate output",
            &evidence.join("manual").join("sneaky"),
            &protected,
        )
        .expect_err("must refuse");
        assert!(err.to_string().contains("path-overlap"), "{err}");
        assert!(err.to_string().contains("manual"), "{err}");
    }

    #[test]
    fn a_sibling_output_is_accepted() {
        let evidence = sandbox("protected-ok");
        let out = sandbox("protected-ok-out");
        let protected = protected_under(&evidence);
        assert_output_is_safe("the candidate output", &out, &protected).expect("disjoint");
    }

    // Junction/symlink creation needs privilege or Developer Mode on Windows; the case is skipped
    // rather than silently passing when the platform refuses to create the link.
    #[test]
    fn a_junction_aliasing_a_protected_root_is_refused_when_one_can_be_created() {
        let evidence = sandbox("junction-evidence");
        fs::create_dir_all(evidence.join("manual")).expect("dirs");
        let base = sandbox("junction-base");
        let link = base.join("alias");

        let created = make_dir_link(&evidence.join("manual"), &link);
        if !created {
            eprintln!("skipping: this platform/session cannot create a directory link");
            return;
        }

        let protected = protected_under(&evidence);
        // Both defences fire: the link component is refused outright, and had it not been, the
        // resolved path would land inside the protected root.
        let err = assert_output_is_safe("the candidate output", &link.join("x"), &protected)
            .expect_err("must refuse");
        let text = err.to_string();
        assert!(
            text.contains("path-reparse-point") || text.contains("path-overlap"),
            "{text}"
        );
        // The protected bytes are untouched.
        assert!(evidence.join("manual").is_dir());
    }

    #[test]
    fn a_directory_is_refused_as_a_file_mutation_target() {
        let root = sandbox("dir-as-file");
        fs::create_dir_all(root.join("out")).expect("dirs");
        let err = resolve_file_for_write(&root.join("out")).expect_err("must refuse");
        assert!(
            err.to_string().contains("path-unexpected-entry-type"),
            "{err}"
        );
    }

    #[test]
    fn an_ordinary_new_file_and_an_ordinary_overwrite_are_both_accepted() {
        let root = sandbox("ordinary-file");
        write_file(&root.join("a.csv"), b"one").expect("create");
        write_file(&root.join("a.csv"), b"two").expect("overwrite");
        assert_eq!(fs::read(root.join("a.csv")).expect("read"), b"two");
    }

    #[test]
    fn an_existing_file_link_is_refused_as_a_mutation_target() {
        let root = sandbox("file-link");
        let sentinel = root.join("protected.txt");
        fs::write(&sentinel, b"protected bytes").expect("write");
        let link = root.join("output.csv");
        if !make_file_link(&sentinel, &link) {
            eprintln!("skipping: this platform/session cannot create a file link");
            return;
        }
        let err = write_file(&link, b"overwritten").expect_err("must refuse");
        assert!(err.to_string().contains("path-reparse-point"), "{err}");
        // The bytes behind the link are untouched.
        assert_eq!(
            fs::read(&sentinel).expect("read"),
            b"protected bytes".to_vec()
        );
    }

    #[test]
    fn a_dangling_link_is_refused_even_though_exists_reports_it_absent() {
        let root = sandbox("dangling");
        let missing = root.join("gone.txt");
        let link = root.join("output.csv");
        if !make_file_link(&missing, &link) {
            eprintln!("skipping: this platform/session cannot create a file link");
            return;
        }
        assert!(!link.exists(), "a dangling link reports itself absent");
        let err = write_file(&link, b"x").expect_err("must refuse");
        assert!(err.to_string().contains("path-reparse-point"), "{err}");
    }

    #[test]
    fn a_layout_carries_every_protected_root_and_cannot_be_emptied() {
        let evidence = sandbox("layout");
        let layout = Layout::new(&evidence).expect("layout");
        assert_eq!(layout.protected().len(), protected_under(&evidence).len());
        assert!(layout
            .assert_output_is_safe("out", &evidence.join("manual").join("x"))
            .is_err());
        assert!(layout
            .assert_output_is_safe("out", &sandbox("layout-out"))
            .is_ok());
    }

    #[cfg(windows)]
    fn make_file_link(target: &Path, link: &Path) -> bool {
        std::os::windows::fs::symlink_file(target, link).is_ok()
            || std::process::Command::new("cmd")
                .args(["/C", "mklink"])
                .arg(link)
                .arg(target)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    #[cfg(not(windows))]
    fn make_file_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    #[cfg(windows)]
    fn make_dir_link(target: &Path, link: &Path) -> bool {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
            || std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    #[cfg(not(windows))]
    fn make_dir_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}
