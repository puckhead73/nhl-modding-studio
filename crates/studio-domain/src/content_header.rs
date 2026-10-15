//! Recomp/Xenia **content-header sidecars** — the save-detection metadata next to the content tree.
//!
//! The NHL Legacy Recomp (Xbox-derived) enumerates saves through per-item sidecar files, not by
//! scanning the content folders:
//!
//! ```text
//! <content>/<PROFILE16>/<TITLE8>/00000001/<NAME>/<NAME>          ← the save container itself
//! <content>/<PROFILE16>/<TITLE8>/Headers/00000001/<NAME>.header  ← 328-byte sidecar (the index)
//! ```
//!
//! A save file **without** its `.header` is invisible to the game (verified on a real tree: the
//! game listed exactly the folders with sidecars, skipped a bit-perfect container without one, and
//! tolerated stale sidecars whose folders were gone). So whenever the studio writes a save into
//! such a tree it must ensure the sidecar exists — otherwise an edit that re-homes or renames a
//! save silently un-registers it.
//!
//! Sidecar layout (verified byte-for-byte against game-written headers, 0x148 bytes):
//!
//! | offset | size  | content                                                            |
//! |--------|-------|--------------------------------------------------------------------|
//! | 0x000  | 4     | content type, big-endian (1 = save)                                |
//! | 0x004  | 4     | constant 1                                                         |
//! | 0x008  | 0x100 | display name, UTF-16BE, NUL-terminated (stale bytes may follow)    |
//! | 0x108  | 0x30  | content name = the save folder name, ASCII, NUL-terminated         |
//! | 0x138  | 8     | profile id (the 16-hex-digit content dir name)                     |
//! | 0x140  | 4     | title id (the 8-hex-digit title dir name)                          |
//! | 0x144  | 4     | 0                                                                  |
//!
//! There is no size, hash, or checksum — the sidecar is pure metadata, so it can be synthesized.
//! When a sibling sidecar of the same save type exists (even a stale one) it is used as the
//! template with only the content name patched, which preserves the game's own display strings;
//! otherwise a header is built from scratch from the layout above.

use std::path::{Path, PathBuf};

use crate::error::DomainError;
use crate::Result;

const HEADER_LEN: usize = 0x148;
const NAME_OFFSET: usize = 0x108;
const NAME_REGION: usize = 0x30;
const DISPLAY_OFFSET: usize = 0x008;
const DISPLAY_REGION: usize = 0x100;
const PROFILE_OFFSET: usize = 0x138;
const TITLE_OFFSET: usize = 0x140;

/// Is `s` exactly `len` ASCII-hex digits (the shape of the profile / title dir names)?
fn is_hex_name(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The header a template was cloned from: the sidecar file and its identity at planning time. A
/// consumer that defers the write (the S2-C copied-target transaction) binds this input so a
/// template that changes between planning and apply invalidates the plan instead of installing
/// bytes derived from a file that no longer exists in that form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateInput {
    pub path: PathBuf,
    pub identity: tdb_txn::FileIdentity,
}

/// What a save file needs in the way of a content-header sidecar. Produced by
/// [`plan_content_header`], which only **reads** the tree; the caller decides when (and through
/// which writer) the bytes land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderPlan {
    /// The path is not inside a header-indexed content tree (a plain save folder, a content-shaped
    /// tree without a `Headers/<type>` index, a name that does not fit the sidecar). Nothing to do.
    NotIndexed,
    /// The sidecar already exists at this path. It is preserved as-is — never rewritten.
    Present(PathBuf),
    /// The sidecar is missing; these are the exact bytes to place at `header_path`. `template` names
    /// the same-type sibling the bytes were cloned from (with only the content name patched), or
    /// `None` when the header was synthesized from the verified layout alone.
    Required {
        header_path: PathBuf,
        bytes: Vec<u8>,
        template: Option<TemplateInput>,
    },
}

impl HeaderPlan {
    /// The bytes and path when a header must be written.
    pub fn required(&self) -> Option<(&Path, &[u8])> {
        match self {
            HeaderPlan::Required {
                header_path, bytes, ..
            } => Some((header_path, bytes)),
            _ => None,
        }
    }
}

/// Plan the content-header sidecar for `save_file` without writing anything. The path must have
/// the shape `<content>/<PROFILE16>/<TITLE8>/<TYPEDIR>/<NAME>/<file>` and the title directory must
/// carry a `Headers/<TYPEDIR>` index for a header to be required at all — the planner never invents
/// the layout. When a same-type sibling sidecar exists it is cloned as the template and reported as
/// a bound input; otherwise the header is synthesized from the verified layout.
pub fn plan_content_header(save_file: &Path) -> Result<HeaderPlan> {
    let Some(save_dir) = save_file.parent() else {
        return Ok(HeaderPlan::NotIndexed);
    };
    let Some(name) = save_dir.file_name().and_then(|n| n.to_str()) else {
        return Ok(HeaderPlan::NotIndexed);
    };
    let Some(type_dir) = save_dir.parent() else {
        return Ok(HeaderPlan::NotIndexed);
    };
    let Some(type_name) = type_dir.file_name().and_then(|n| n.to_str()) else {
        return Ok(HeaderPlan::NotIndexed);
    };
    let Some(title_dir) = type_dir.parent() else {
        return Ok(HeaderPlan::NotIndexed);
    };
    let (Some(title_name), Some(profile_name)) = (
        title_dir.file_name().and_then(|n| n.to_str()),
        title_dir
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str()),
    ) else {
        return Ok(HeaderPlan::NotIndexed);
    };
    if !is_hex_name(title_name, 8) || !is_hex_name(profile_name, 16) {
        return Ok(HeaderPlan::NotIndexed); // not a content tree
    }
    // The content name must fit its fixed region (with its NUL) and be plain ASCII.
    if name.len() >= NAME_REGION || !name.is_ascii() {
        return Ok(HeaderPlan::NotIndexed);
    }

    // Only trees that actually index through sidecars get one — never invent the layout.
    let headers_dir = title_dir.join("Headers").join(type_name);
    if !headers_dir.is_dir() {
        return Ok(HeaderPlan::NotIndexed);
    }
    let header_path = headers_dir.join(format!("{name}.header"));
    if header_path.exists() {
        return Ok(HeaderPlan::Present(header_path));
    }

    let (bytes, template) = match template_header(&headers_dir, name) {
        Some((template_path, mut t)) => {
            let identity = tdb_txn::FileIdentity::of_bytes(&t);
            patch_name(&mut t, name);
            (
                t,
                Some(TemplateInput {
                    path: template_path,
                    identity,
                }),
            )
        }
        None => (synthesize_header(name, profile_name, title_name), None),
    };
    Ok(HeaderPlan::Required {
        header_path,
        bytes,
        template,
    })
}

/// Ensure the content-header sidecar for `save_file` exists, creating it if the file sits inside a
/// recomp/Xenia content tree that carries a `Headers` index. Returns the sidecar path when one was
/// created, `None` when nothing needed doing (sidecar already present, or the path is not inside a
/// header-indexed content tree — a plain save folder stays untouched). The direct writer over
/// [`plan_content_header`]; the transaction path consumes the plan instead.
pub fn ensure_content_header(save_file: &Path) -> Result<Option<PathBuf>> {
    match plan_content_header(save_file)? {
        HeaderPlan::NotIndexed | HeaderPlan::Present(_) => Ok(None),
        HeaderPlan::Required {
            header_path, bytes, ..
        } => {
            std::fs::write(&header_path, bytes).map_err(|source| DomainError::Io {
                path: header_path.display().to_string(),
                source,
            })?;
            Ok(Some(header_path))
        }
    }
}

/// The save-type prefix of a content name (`"ROSTER 20260723000649"` → `"ROSTER"`).
fn type_prefix(name: &str) -> &str {
    name.split(' ').next().unwrap_or(name)
}

/// A sibling sidecar of the same save type to clone (any vintage — stale ones are fine, the game's
/// display strings inside are what we want to preserve), with the path it was read from.
fn template_header(headers_dir: &Path, name: &str) -> Option<(PathBuf, Vec<u8>)> {
    let prefix = type_prefix(name);
    let entries = std::fs::read_dir(headers_dir).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "header")
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| type_prefix(s) == prefix)
        })
        .collect();
    candidates.sort(); // deterministic pick
    for p in candidates {
        if let Ok(bytes) = std::fs::read(&p) {
            if bytes.len() == HEADER_LEN {
                return Some((p, bytes));
            }
        }
    }
    None
}

/// Overwrite the ASCII content-name region (zero-filled, so no stale tail survives the patch).
fn patch_name(header: &mut [u8], name: &str) {
    let region = &mut header[NAME_OFFSET..NAME_OFFSET + NAME_REGION];
    region.fill(0);
    region[..name.len()].copy_from_slice(name.as_bytes());
}

/// Build a sidecar from scratch per the verified layout. The display name falls back to the save
/// type prefix (only cosmetic — templates preserve the game's own strings when any exist).
fn synthesize_header(name: &str, profile_hex: &str, title_hex: &str) -> Vec<u8> {
    let mut h = vec![0u8; HEADER_LEN];
    h[0x00..0x04].copy_from_slice(&1u32.to_be_bytes()); // content type: save
    h[0x04..0x08].copy_from_slice(&1u32.to_be_bytes());
    // Display name, UTF-16BE.
    let display = type_prefix(name);
    let mut off = DISPLAY_OFFSET;
    for u in display.encode_utf16().take(DISPLAY_REGION / 2 - 1) {
        h[off..off + 2].copy_from_slice(&u.to_be_bytes());
        off += 2;
    }
    patch_name(&mut h, name);
    // Profile + title ids from the directory names (validated hex by the caller).
    for (i, chunk) in profile_hex.as_bytes().chunks(2).enumerate().take(8) {
        h[PROFILE_OFFSET + i] =
            u8::from_str_radix(std::str::from_utf8(chunk).unwrap_or("0"), 16).unwrap_or(0);
    }
    for (i, chunk) in title_hex.as_bytes().chunks(2).enumerate().take(4) {
        h[TITLE_OFFSET + i] =
            u8::from_str_radix(std::str::from_utf8(chunk).unwrap_or("0"), 16).unwrap_or(0);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesized_layout_matches_spec() {
        let h = synthesize_header("ROSTER 20260723000649", "B13EBABEBABEBABE", "454109EC");
        assert_eq!(h.len(), HEADER_LEN);
        assert_eq!(&h[0x00..0x08], &[0, 0, 0, 1, 0, 0, 0, 1]);
        // UTF-16BE "ROSTER".
        assert_eq!(&h[0x08..0x14], b"\0R\0O\0S\0T\0E\0R");
        assert_eq!(&h[0x14..0x16], &[0, 0]);
        assert_eq!(&h[0x108..0x11D], b"ROSTER 20260723000649");
        assert_eq!(h[0x11D], 0);
        assert_eq!(
            &h[0x138..0x140],
            &[0xB1, 0x3E, 0xBA, 0xBE, 0xBA, 0xBE, 0xBA, 0xBE]
        );
        assert_eq!(&h[0x140..0x144], &[0x45, 0x41, 0x09, 0xEC]);
        assert_eq!(&h[0x144..0x148], &[0, 0, 0, 0]);
    }

    #[test]
    fn patch_name_zero_fills_stale_tail() {
        let mut h = vec![0xAAu8; HEADER_LEN];
        patch_name(&mut h, "ROSTER 1");
        assert_eq!(&h[0x108..0x110], b"ROSTER 1");
        assert!(
            h[0x110..0x138].iter().all(|&b| b == 0),
            "stale tail cleared"
        );
        assert_eq!(h[0x107], 0xAA, "bytes before the region untouched");
        assert_eq!(h[0x138], 0xAA, "bytes after the region untouched");
    }
}
