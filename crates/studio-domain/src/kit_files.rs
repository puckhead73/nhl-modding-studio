//! Phase 2.6 (gen7) — a team's **kits** joined to the textures that dress them.
//!
//! Research 2026-09-21 pm handoff R56 (`research-r56-gen7-kit-selectors`) measured the join on
//! NHL 11 → Legacy, both consoles: a `stockteamjerseys` row's `(style, teamartid, variant)` triple
//! names the kit's files, and it does so for **every** row of every title (627 / 676 / 684 / 704 /
//! 659 / 703 — 100 % on each):
//!
//! ```text
//! rendering/jersey/texlib_<style>_<teamartid>_<variant>.rx2     (.rpsgl on PS3)
//! rendering/pant/texlib_<style>_<teamartid>_<variant>.rx2
//! rendering/sock/…                                              (grammar per title, below)
//! ```
//!
//! ## The contract is decided by the title, the evidence by the tree (coordinator F1, 22 Sep 2026)
//!
//! The first version of this module decided *which components a kit needs* by looking at the
//! directory listing — and so a tree with no `sock/` directory made every jersey + pant pair a
//! "complete" kit and a "complete spare". The coordinator's review reproduced that with an
//! existence-only tree and named it a P2 defect: **missing, unreadable or unrecognized data must
//! yield incomplete or unknown evidence, never complete spares.**
//!
//! The rule is now in two separate parts:
//!
//! 1. [`kit_contract`] says what the *measured* title / platform requires — the file grammar of
//!    each component, from R56 and the 22 Sep 2026 real-tree measurement — independently of what
//!    is on disk. A title outside the measured set has an *unmeasured* contract and nothing is
//!    called complete on it.
//! 2. [`RenderingListing`] reads the tree once per component directory and records, per directory,
//!    whether it was listed, is absent, or could not be read. Every component then carries
//!    `present` / `missing` / `unknown` evidence, and [`completeness`] — the **one** rule both the
//!    kit rows and the shipped spare pool use — is `complete` only when every required component
//!    is present, `incomplete` when any is definitely missing, and `unknown` otherwise.
//!
//! ## Two sock grammars, both measured (22 Sep 2026)
//!
//! The jersey / pant template holds on every title — file counts on the real trees match R56's
//! census exactly (NHL 11 753, NHL 12 967, NHL 13 864, NHL 14 1 024, NHL 15 683, Legacy 905).
//! Socks come in **two** forms:
//!
//! ```text
//! NHL 13 → Legacy   sock_<style>_<teamartid>_<variant>_cm            one file per kit
//! NHL 11 / 12       sock_<style>_<0|1>_<teamartid>_<variant>_cm      two files per kit
//! ```
//!
//! The second is R56 §1's "two per kit" note, confirmed on the real NHL 11 tree: 1 512 `_cm` sock
//! files = 2 × the 756 the census counted. On a two-file title **both** files are required
//! components and both are listed separately; what the `0` / `1` sub-index selects is **not**
//! measured, which is one of the reasons re-pointing a row stays unsupported here.
//!
//! Alternatives the research ruled out are not implemented: `jerseyindex` in place of `variant`
//! resolves only 457–534 of the rows (it is the per-team ordinal, not the file variant), and
//! `fevariant` equals `variant` on every row, so it adds nothing.
//!
//! Beyond the rows, the same directories hold kits with **no row** — 26 to 322 per title. Those are
//! listed per team art id as *shipped spare candidates*, each with the same per-component evidence
//! and the same completeness verdict as a row. Only a *complete* candidate could ever be a
//! reassignment target, and reassignment itself has no write contract yet.
//!
//! ## Refusals (measured, not guessed)
//!
//! * **NHL 09 / 10** ship no `stockteamjerseys` table at all. Kit files exist; how the game selects
//!   among them is unknown, so nothing is listed rather than a guess presented as a selector.
//! * **Custom teams** (`customteamjerseys`, art id 20054) have 60 rows and **0** resolve to files:
//!   custom kits are not per-team files. Refused with that reason.
//! * A row whose `teamartid` is not its team's `artid` is a refusal, not a silent re-key — the
//!   research measured them consistent on every row of every title.
//!
//! Nothing here is a writer, and nothing claims what the console draws for a given kit.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project::Platform;

/// The art id `customteamjerseys` uses; it has no per-team files (R56 §1).
pub const CUSTOM_TEAM_ART_ID: u64 = 20054;

// ---------------------------------------------------------------------------
// The contract: what a measured title requires, independent of the tree
// ---------------------------------------------------------------------------

/// The measured sock filename grammar of a title.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SockGrammar {
    /// NHL 13 → Legacy (and Recompiled): `sock_<style>_<artid>_<variant>_cm`, one file per kit.
    SingleFile,
    /// NHL 11 / 12: `sock_<style>_<0|1>_<artid>_<variant>_cm`, two files per kit.
    TwoFile,
}

impl SockGrammar {
    fn describe(self) -> &'static str {
        match self {
            Self::SingleFile => "sock_<style>_<art id>_<variant>_cm — one sock file per kit (NHL 13 → Legacy)",
            Self::TwoFile => "sock_<style>_<0|1>_<art id>_<variant>_cm — two sock files per kit (NHL 11 / 12): a mirrored pair, the `_1_` file the horizontal mirror of `_0_` (research R74, measured on every pair of both titles); one kit, both files required; which leg each dresses at runtime is not claimed",
        }
    }
    /// The sock sub-indices a kit requires under this grammar.
    fn subs(self) -> &'static [Option<u64>] {
        match self {
            Self::SingleFile => &[None],
            Self::TwoFile => &[Some(0), Some(1)],
        }
    }
}

/// What the title / platform's kit files are measured to look like — or why nothing is measured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum KitContract {
    /// The join is measured for this title: the three families and the sock grammar.
    Measured {
        sock: SockGrammar,
        /// The container extension the platform ships (`rx2` / `rpsgl`).
        extension: String,
        /// The measurement the contract rests on.
        source: String,
    },
    /// The title has no `stockteamjerseys` table: files exist, the selector is unknown.
    NoSelectorTable { reason: String },
    /// No measurement covers this title / platform: nothing is joined, nothing is called complete.
    Unmeasured { reason: String },
}

impl KitContract {
    pub fn describe(&self) -> String {
        match self {
            Self::Measured { sock, extension, source } => {
                format!("jersey `texlib_<style>_<art id>_<variant>.{extension}`, pants the same under `pant/`, socks `{}` ({source})", sock.describe())
            }
            Self::NoSelectorTable { reason } | Self::Unmeasured { reason } => reason.clone(),
        }
    }
}

/// The measured contract for a title / platform. Decided from the title identity alone — never
/// from the files on disk — so a tree that is missing something reports that something as missing
/// rather than quietly lowering the bar.
pub fn kit_contract(game_id: Option<&str>, platform: Platform) -> KitContract {
    let extension = match platform {
        Platform::Ps3 => "rpsgl",
        Platform::Xbox | Platform::Recomp => "rx2",
        Platform::Ps4 => {
            return KitContract::Unmeasured {
                reason: "PS4 kit textures are not covered by the gen7 kit-selector measurement (research R56 measured NHL 11 → Legacy on Xbox 360 / PS3 only)".into(),
            }
        }
    };
    match game_id {
        Some("nhl11" | "nhl12") => KitContract::Measured {
            sock: SockGrammar::TwoFile,
            extension: extension.into(),
            source: "research 2026-09-21 pm handoff R56 §1; two-file sock form measured on the real NHL 11 / 12 trees 22 Sep 2026".into(),
        },
        Some("nhl13" | "nhl14" | "nhl15" | "nhllegacy") => KitContract::Measured {
            sock: SockGrammar::SingleFile,
            extension: extension.into(),
            source: "research 2026-09-21 pm handoff R56 §1 (100 % of rows on NHL 13 → Legacy)".into(),
        },
        Some("nhl09" | "nhl10") => KitContract::NoSelectorTable {
            reason: "NHL 09 / 10 ship no `stockteamjerseys` table: kit textures exist under `rendering/`, but how the game selects among them is not measured, so no join is offered (research R56)".into(),
        },
        Some(other) => KitContract::Unmeasured {
            reason: format!("the kit-file grammar of `{other}` is not measured (research R56 covers NHL 11 → Legacy); rows are listed without textures and nothing is called complete"),
        },
        None => KitContract::Unmeasured {
            reason: "the open project has no recorded title, so the kit-file grammar cannot be chosen; rows are listed without textures and nothing is called complete".into(),
        },
    }
}

/// Whether the nameplate naming `name_<style>_<art id>_<variant>_cm` is measured for a title /
/// platform (research R101 as corrected by R103, 26 Sep 2026: every row resolves and every
/// discriminating row prefers the jersey triple on X360 / PS3 Legacy, X360 / PS3 NHL 15, PS3 NHL 12
/// and 13). A naming association, not the game's runtime selection; other titles are not looked up.
pub fn nameplate_measured(game_id: Option<&str>, platform: Platform) -> bool {
    matches!(
        (game_id, platform),
        (Some("nhllegacy" | "nhl15"), Platform::Xbox | Platform::Ps3) | (Some("nhl12" | "nhl13"), Platform::Ps3)
    )
}

/// The nameplate sheet's stem for a kit triple (R101 / R103) — under `jersey/`.
pub fn nameplate_stem(style: u64, art_id: u64, variant: u64) -> String {
    format!("name_{style}_{art_id}_{variant}_cm")
}

// ---------------------------------------------------------------------------
// Parts, stems and the tree listing
// ---------------------------------------------------------------------------

/// One of the three texture families a kit needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KitPart {
    Jersey,
    Pant,
    Sock,
}

impl KitPart {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jersey => "jersey",
            Self::Pant => "pant",
            Self::Sock => "sock",
        }
    }

    /// The `rendering/` sub-directory that holds this family.
    pub fn dir(self) -> &'static str {
        self.as_str()
    }

    /// The file stem for a triple, per the measured templates. `sub` selects the two-file sock
    /// form; `None` is the single-file form and is ignored by the other families.
    fn file_stem(self, style: u64, art_id: u64, variant: u64, sub: Option<u64>) -> String {
        match (self, sub) {
            (Self::Jersey, _) | (Self::Pant, _) => format!("texlib_{style}_{art_id}_{variant}"),
            (Self::Sock, None) => format!("sock_{style}_{art_id}_{variant}_cm"),
            (Self::Sock, Some(n)) => format!("sock_{style}_{n}_{art_id}_{variant}_cm"),
        }
    }

    /// Parse a stem back into `(style, art id, variant, sub)` under a grammar, or `None` when the
    /// name is not one this family's template produces. Exact inverse of [`Self::file_stem`].
    fn parse_stem(self, sock: SockGrammar, stem: &str) -> Option<(u64, u64, u64, Option<u64>)> {
        let parts: Vec<&str> = stem.split('_').collect();
        let num = |s: &str| -> Option<u64> {
            // Reject signs, leading zeros and empty tokens: the writer never produces them.
            if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            s.parse().ok()
        };
        let parsed = match (self, sock) {
            (Self::Jersey | Self::Pant, _) => match parts.as_slice() {
                ["texlib", s, a, v] => (num(s)?, num(a)?, num(v)?, None),
                _ => return None,
            },
            (Self::Sock, SockGrammar::SingleFile) => match parts.as_slice() {
                ["sock", s, a, v, "cm"] => (num(s)?, num(a)?, num(v)?, None),
                _ => return None,
            },
            (Self::Sock, SockGrammar::TwoFile) => match parts.as_slice() {
                ["sock", s, n, a, v, "cm"] => {
                    let n = num(n)?;
                    if n > 1 {
                        return None;
                    }
                    (num(s)?, num(a)?, num(v)?, Some(n))
                }
                _ => return None,
            },
        };
        // Round-trip guard: only accept a stem the template would write back identically.
        (self.file_stem(parsed.0, parsed.1, parsed.2, parsed.3) == stem).then_some(parsed)
    }
}

/// How a component directory of the tree read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DirEvidence {
    /// The directory was listed; `files` is the number of entries with a kit container extension.
    Listed { files: usize },
    /// The directory does not exist: its components are definitely missing.
    Absent,
    /// The directory exists but could not be read (permissions, not a directory, I/O): its
    /// components are unknown, not missing.
    Unreadable { reason: String },
    /// The directory is not loose in the tree; its members were listed from the named render
    /// archive(s) beside the tree (PS3 NHL 12 / 13 keep `rendering/` inside `cacherender.big` /
    /// `nocacherender.big`, `EB\0\3` v`0x00000400`). `files` counts the kit-container members
    /// under this part; a member's presence is evidence from the archive's file table only — the
    /// bytes are not decoded here.
    Archived { archive: String, files: usize },
}

/// One listed kit container: its file name and, when it lives in an archive, the archive name.
#[derive(Debug, Clone)]
struct ListedFile {
    name: String,
    archive: Option<String>,
}

/// The render archives PS3 NHL 12 / 13 ship beside `rendering/` (research 2026-09-21 pm handoff
/// R56 measured those two titles' kits by archive member listing; the canonical D1 inventory
/// records every member under `rendering/`, all `chunklzx`).
pub const RENDER_ARCHIVES: [&str; 2] = ["cacherender.big", "nocacherender.big"];

/// List the member paths of an `EB\0\3` archive from its front tables only (header + file /
/// name / directory tables; no payload is read). Errors name the archive and the parse failure.
pub fn list_eb_members(path: &Path) -> std::result::Result<Vec<String>, String> {
    use std::io::{Read, Seek, SeekFrom};
    use tdb_archive_big::eb::{self, EbHeader};
    let label = path.display().to_string();
    let mut file = std::fs::File::open(path).map_err(|e| format!("{label}: {e}"))?;
    let mut header_buf = vec![0u8; eb::HEADER_LEN];
    file.read_exact(&mut header_buf).map_err(|e| format!("{label}: header: {e}"))?;
    let header = EbHeader::parse(&header_buf).map_err(|e| format!("{label}: {e}"))?;
    let mut table_buf = vec![0u8; header.table_end()];
    file.seek(SeekFrom::Start(0)).map_err(|e| format!("{label}: {e}"))?;
    file.read_exact(&mut table_buf).map_err(|e| format!("{label}: tables: {e}"))?;
    let archive = eb::EbArchive::parse(&table_buf).map_err(|e| format!("{label}: {e}"))?;
    Ok(archive.entries.into_iter().map(|e| e.path).collect())
}

/// The tree's component directories, read once. Stems are keyed with the file name they resolve
/// to so the platform's own extension wins when a mixed tree carries both.
#[derive(Debug, Clone)]
pub struct RenderingListing {
    root: PathBuf,
    extension: String,
    dirs: BTreeMap<KitPart, (DirEvidence, BTreeMap<String, ListedFile>)>,
}

impl RenderingListing {
    /// [`RenderingListing::read`], then — for every part whose loose directory is **absent** —
    /// the render archives beside the tree (`<root>/../cacherender.big`, `nocacherender.big`)
    /// when they exist: their file tables are listed and the part's members recorded as
    /// [`DirEvidence::Archived`]. A part that is loose keeps its loose evidence; an archive that
    /// exists but does not parse makes the part **unreadable** (unknown), never absent; no archive
    /// at all leaves the part absent as before.
    pub fn read_with_archives(root: &Path, extension: &str) -> Self {
        let mut listing = Self::read(root, extension);
        let needs_archive = listing.dirs.values().any(|(e, _)| matches!(e, DirEvidence::Absent));
        if !needs_archive {
            return listing;
        }
        let Some(parent) = root.parent() else { return listing };
        let present: Vec<PathBuf> = RENDER_ARCHIVES.iter().map(|a| parent.join(a)).filter(|p| p.is_file()).collect();
        if present.is_empty() {
            return listing;
        }
        let mut members: Vec<(String, String)> = Vec::new(); // (archive name, member path)
        let mut failures: Vec<String> = Vec::new();
        for p in &present {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            match list_eb_members(p) {
                Ok(paths) => members.extend(paths.into_iter().map(|m| (name.clone(), m))),
                Err(reason) => failures.push(reason),
            }
        }
        let archive_label = present.iter().filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).collect::<Vec<_>>().join(" + ");
        for part in [KitPart::Jersey, KitPart::Pant, KitPart::Sock] {
            if !matches!(listing.dirs[&part].0, DirEvidence::Absent) {
                continue;
            }
            if !failures.is_empty() {
                listing.dirs.insert(part, (DirEvidence::Unreadable { reason: format!("the render archive could not be listed: {}", failures.join("; ")) }, BTreeMap::new()));
                continue;
            }
            let prefix = format!("rendering/{}/", part.dir());
            let mut stems: BTreeMap<String, ListedFile> = BTreeMap::new();
            for (archive, m) in &members {
                let norm = m.replace('\\', "/");
                let Some(rest) = norm.strip_prefix(&prefix) else { continue };
                if rest.contains('/') {
                    continue;
                }
                let Some((stem, ext)) = rest.rsplit_once('.') else { continue };
                if ext != "rx2" && ext != "rpsgl" {
                    continue;
                }
                match stems.get(stem) {
                    Some(existing) if existing.name.ends_with(&format!(".{extension}")) => {}
                    _ => {
                        stems.insert(stem.to_string(), ListedFile { name: rest.to_string(), archive: Some(archive.clone()) });
                    }
                }
            }
            listing.dirs.insert(part, (DirEvidence::Archived { archive: archive_label.clone(), files: stems.len() }, stems));
        }
        listing
    }

    /// Read `rendering/{jersey,pant,sock}` under `root`. Never fails: every outcome is evidence.
    pub fn read(root: &Path, extension: &str) -> Self {
        let mut dirs = BTreeMap::new();
        for part in [KitPart::Jersey, KitPart::Pant, KitPart::Sock] {
            let dir = root.join(part.dir());
            let evidence = match std::fs::read_dir(&dir) {
                Ok(rd) => {
                    let mut stems: BTreeMap<String, ListedFile> = BTreeMap::new();
                    let mut unreadable: Option<String> = None;
                    for entry in rd {
                        let entry = match entry {
                            Ok(e) => e,
                            Err(e) => {
                                unreadable = Some(e.to_string());
                                break;
                            }
                        };
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let Some((stem, ext)) = name.rsplit_once('.') else { continue };
                        if ext != "rx2" && ext != "rpsgl" {
                            continue;
                        }
                        // PS3 ships `.rpsgl`, Xbox `.rx2`; a mixed tree is tolerated, the platform's
                        // extension preferred.
                        match stems.get(stem) {
                            Some(existing) if existing.name.ends_with(&format!(".{extension}")) => {}
                            _ => {
                                stems.insert(stem.to_string(), ListedFile { name: name.clone(), archive: None });
                            }
                        }
                    }
                    match unreadable {
                        Some(reason) => (DirEvidence::Unreadable { reason: format!("{}: listing stopped: {reason}", dir.display()) }, BTreeMap::new()),
                        None => (DirEvidence::Listed { files: stems.len() }, stems),
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (DirEvidence::Absent, BTreeMap::new()),
                Err(e) => (DirEvidence::Unreadable { reason: format!("{}: {e}", dir.display()) }, BTreeMap::new()),
            };
            dirs.insert(part, evidence);
        }
        Self { root: root.to_path_buf(), extension: extension.to_string(), dirs }
    }

    /// The per-directory evidence, in family order.
    pub fn directories(&self) -> Vec<(KitPart, DirEvidence)> {
        self.dirs.iter().map(|(p, (e, _))| (*p, e.clone())).collect()
    }

    fn component(&self, part: KitPart, label: &str, stem: String) -> KitComponent {
        let (evidence, stems) = &self.dirs[&part];
        let (status, path, archived, reason) = match evidence {
            DirEvidence::Listed { .. } => match stems.get(&stem) {
                Some(f) => (ComponentStatus::Present, Some(self.root.join(part.dir()).join(&f.name).to_string_lossy().into_owned()), None, None),
                None => (ComponentStatus::Missing, None, None, Some(format!("`{}/{stem}.{}` is not in the listed directory", part.dir(), self.extension))),
            },
            DirEvidence::Archived { archive, .. } => match stems.get(&stem) {
                Some(f) => (
                    ComponentStatus::Present,
                    None,
                    Some((f.archive.clone().unwrap_or_else(|| archive.clone()), format!("rendering/{}/{}", part.dir(), f.name))),
                    None,
                ),
                None => (ComponentStatus::Missing, None, None, Some(format!("`rendering/{}/{stem}.{}` is not among the members of {archive}", part.dir(), self.extension))),
            },
            DirEvidence::Absent => (ComponentStatus::Missing, None, None, Some(format!("the `{}/` directory is absent from the rendering tree", part.dir()))),
            DirEvidence::Unreadable { reason } => (ComponentStatus::Unknown, None, None, Some(format!("the `{}/` directory could not be read ({reason}); this component is unknown, not absent", part.dir()))),
        };
        let (archive, member_path) = match &archived {
            Some((a, m)) => (Some(a.clone()), Some(m.clone())),
            None => (None, None),
        };
        let archive_member = archived.as_ref().map(|(a, m)| format!("{a}:{m}"));
        KitComponent { part, label: label.to_string(), file_stem: stem, path, archive_member, archive, member_path, status, reason }
    }

    /// Every `(style, variant)` of `art_id` that any family names, parsed under `sock` grammar.
    fn observed_triples(&self, art_id: u64, sock: SockGrammar) -> BTreeSet<(u64, u64)> {
        let mut out = BTreeSet::new();
        for (part, (_, stems)) in &self.dirs {
            for stem in stems.keys() {
                if let Some((style, art, variant, _)) = part.parse_stem(sock, stem) {
                    if art == art_id {
                        out.insert((style, variant));
                    }
                }
            }
        }
        out
    }

    /// Sock stems of `art_id` written in the *other* grammar — an observation the contract does
    /// not join, reported so a disagreement between the title contract and the tree is visible.
    fn sock_stems_in_other_grammar(&self, art_id: u64, sock: SockGrammar) -> usize {
        let other = match sock {
            SockGrammar::SingleFile => SockGrammar::TwoFile,
            SockGrammar::TwoFile => SockGrammar::SingleFile,
        };
        self.dirs[&KitPart::Sock]
            .1
            .keys()
            .filter(|stem| KitPart::Sock.parse_stem(other, stem).is_some_and(|(_, a, _, _)| a == art_id))
            .count()
    }
}

// ---------------------------------------------------------------------------
// Components, completeness — the one rule
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    /// The file is in the tree.
    Present,
    /// The file is definitely not there (the directory was listed, or does not exist).
    Missing,
    /// Studio could not tell (no tree attached, directory unreadable, contract unmeasured).
    Unknown,
}

/// One required component of a kit, with where Studio looked and what it found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitComponent {
    pub part: KitPart,
    /// Human label: `jersey`, `pants`, `socks`, `socks (file 0)`, `socks (file 1)`.
    pub label: String,
    /// The stem the contract expects (no extension).
    pub file_stem: String,
    /// Resolved absolute path when present as a loose file.
    pub path: Option<String>,
    /// `<archive>:<member path>` when present as a render-archive member (PS3 NHL 12 / 13); the
    /// member is listed from the archive's file table, not decoded here.
    pub archive_member: Option<String>,
    /// The render archive's file name (`cacherender.big`) and the member path inside it, as two
    /// fields for the preview route (never re-parsed from `archive_member`).
    pub archive: Option<String>,
    pub member_path: Option<String>,
    pub status: ComponentStatus,
    /// Why it is missing or unknown; `None` when present.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    /// Every required component is present.
    Complete,
    /// At least one required component is definitely missing.
    Incomplete,
    /// Nothing is definitely missing, but at least one component (or the contract) is unknown.
    Unknown,
}

/// **The** completeness rule, shared by kit rows and spare candidates. An empty component list
/// (no contract) is unknown, never complete.
pub fn completeness(components: &[KitComponent]) -> Completeness {
    if components.is_empty() {
        return Completeness::Unknown;
    }
    if components.iter().any(|c| c.status == ComponentStatus::Missing) {
        Completeness::Incomplete
    } else if components.iter().any(|c| c.status == ComponentStatus::Unknown) {
        Completeness::Unknown
    } else {
        Completeness::Complete
    }
}

/// The required components of a triple under a measured contract, resolved against the listing
/// (or all unknown when no tree is attached).
fn required_components(listing: Option<&RenderingListing>, sock: SockGrammar, style: u64, art_id: u64, variant: u64) -> Vec<KitComponent> {
    let mut specs: Vec<(KitPart, String, Option<u64>)> = vec![(KitPart::Jersey, "jersey".into(), None), (KitPart::Pant, "pants".into(), None)];
    for sub in sock.subs() {
        let label = match sub {
            None => "socks".to_string(),
            Some(n) => format!("socks (file {n})"),
        };
        specs.push((KitPart::Sock, label, *sub));
    }
    specs
        .into_iter()
        .map(|(part, label, sub)| {
            let stem = part.file_stem(style, art_id, variant, sub);
            match listing {
                Some(l) => l.component(part, &label, stem),
                None => KitComponent {
                    part,
                    label,
                    file_stem: stem,
                    path: None,
                    archive_member: None,
                    archive: None,
                    member_path: None,
                    status: ComponentStatus::Unknown,
                    reason: Some("no rendering tree is attached to this project, so the file was not looked for".into()),
                },
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Views
// ---------------------------------------------------------------------------

/// One kit row: the triple, its flags, and the per-component evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitView {
    pub record: u32,
    pub jersey_index: u32,
    pub style: u64,
    pub team_art_id: u64,
    pub variant: u64,
    pub is_default: bool,
    pub is_light_color: bool,
    pub has_tie: bool,
    pub helmet_logo: bool,
    pub helmet_numbers: bool,
    /// Every component the contract requires, with what was found. Empty when the row is refused
    /// or the contract is not measured.
    pub components: Vec<KitComponent>,
    /// The nameplate sheet the row's triple names (R101 / R103) — informational, **not** part of
    /// `completeness`; `None` when the naming is not measured for the title or the row is refused.
    pub nameplate: Option<KitComponent>,
    pub completeness: Completeness,
    /// Present when this row cannot be joined at all, with the measured reason.
    pub refusal: Option<String>,
}

/// A `(style, variant)` some family names under the team's art id but no row points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpareKit {
    pub style: u64,
    pub variant: u64,
    pub components: Vec<KitComponent>,
    /// Same rule as the rows: only a `complete` candidate could ever be re-pointed at.
    pub completeness: Completeness,
}

/// A team's kits, the shipped spare candidates and every piece of evidence behind the verdicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamKitsView {
    pub team_id: u64,
    pub team_art_id: Option<u64>,
    pub contract: KitContract,
    /// The rendering root the evidence was read from, when one is attached.
    pub rendering_root: Option<String>,
    /// Per-directory evidence, in family order; empty when no tree is attached.
    pub directories: Vec<(KitPart, DirEvidence)>,
    pub kits: Vec<KitView>,
    pub spares: Vec<SpareKit>,
    /// Always `false` today: no write contract exists for re-pointing a row.
    pub reassignment_supported: bool,
    pub reassignment_reason: String,
    pub notes: Vec<String>,
}

/// Every team's own `artid`, keyed by team id — the value a kit row's `teamartid` must equal
/// (measured consistent on every shipped row of every title, R56 §1). Empty when the title's team
/// table carries no `artid` column (NHL 07's thin table).
pub fn team_art_ids(db: &tdb_core::TdbFile, meta: &tdb_defs::FieldMetaIndex) -> BTreeMap<u64, u64> {
    let mut out = BTreeMap::new();
    let Some(kott) = db.table_canonical(tdb_relations::entity::ids::TEAM) else { return out };
    let field = |name: &str| {
        kott.fields.iter().find(|f| {
            meta.field(&kott.id.raw(), &f.id.raw())
                .map(|m| m.name.as_str() == name)
                .unwrap_or(false)
        })
    };
    let (Some(f_id), Some(f_art)) = (field("index"), field("artid")) else { return out };
    for rec in 0..kott.current_records as u32 {
        if let (Some(id), Some(art)) = (db.read_u64(kott, rec, f_id), db.read_u64(kott, rec, f_art)) {
            out.insert(id, art);
        }
    }
    out
}

/// Build a team's kit view from its decoded jersey rows.
///
/// `rows` are the team's `stockteamjerseys` rows (`crate::team_tabs::team_jerseys`), `team_art_id`
/// the team's own `artid` from the team table — the value every row's `teamartid` must equal.
/// `contract` comes from [`kit_contract`] and decides what a kit needs; `rendering_root` is where
/// the evidence is read from.
pub fn team_kits(
    team_id: u64,
    team_art_id: Option<u64>,
    rows: &[crate::team_tabs::TeamJersey],
    contract: KitContract,
    rendering_root: Option<&Path>,
    nameplates: bool,
) -> TeamKitsView {
    let (sock, extension) = match &contract {
        KitContract::Measured { sock, extension, .. } => (Some(*sock), extension.as_str()),
        _ => (None, "rx2"),
    };
    let listing = match (rendering_root, sock) {
        (Some(root), Some(_)) => Some(RenderingListing::read_with_archives(root, extension)),
        _ => None,
    };

    let mut kits = Vec::with_capacity(rows.len());
    let mut used: BTreeSet<(u64, u64)> = BTreeSet::new();
    for r in rows {
        let (style, art, variant) = (r.style_raw as u64, r.team_art_id as u64, r.variant_raw as u64);
        let refusal = if art == CUSTOM_TEAM_ART_ID {
            Some("custom-team kits (art id 20054) are not per-team files: none of the 60 shipped custom rows resolves to a texture (research 2026-09-21 pm handoff R56 §1)".to_string())
        } else if team_art_id.is_some_and(|a| a != art) {
            Some(format!(
                "this row's team art id {art} is not the team's own art id {} — the two are consistent on every shipped row of every measured title, so this row is not joined to files",
                team_art_id.unwrap()
            ))
        } else {
            None
        };
        let components = match (&refusal, sock) {
            (None, Some(sock)) => required_components(listing.as_ref(), sock, style, art, variant),
            _ => Vec::new(),
        };
        let verdict = if refusal.is_some() { Completeness::Incomplete } else { completeness(&components) };
        let nameplate = (nameplates && refusal.is_none() && sock.is_some()).then(|| {
            let stem = nameplate_stem(style, art, variant);
            match listing.as_ref() {
                Some(l) => l.component(KitPart::Jersey, "nameplate", stem),
                None => KitComponent {
                    part: KitPart::Jersey,
                    label: "nameplate".into(),
                    file_stem: stem,
                    path: None,
                    archive_member: None,
                    archive: None,
                    member_path: None,
                    status: ComponentStatus::Unknown,
                    reason: Some("no rendering tree is attached to this project, so the file was not looked for".into()),
                },
            }
        });
        used.insert((style, variant));
        kits.push(KitView {
            record: r.record,
            jersey_index: r.jersey_index,
            style,
            team_art_id: art,
            variant,
            is_default: r.is_default,
            is_light_color: r.is_light_color != 0,
            has_tie: r.has_tie != 0,
            helmet_logo: r.helmet_logo != 0,
            helmet_numbers: r.helmet_numbers != 0,
            components,
            nameplate,
            completeness: verdict,
            refusal,
        });
    }

    // The shipped spare candidates: every triple some family names under the team's art id and no
    // row uses — judged by the same rule as the rows, so a jersey-only or sock-less triple is
    // listed as *incomplete*, never as a complete spare.
    let spares: Vec<SpareKit> = match (&listing, team_art_id, sock) {
        (Some(l), Some(art), Some(sock)) if art != CUSTOM_TEAM_ART_ID => l
            .observed_triples(art, sock)
            .into_iter()
            .filter(|t| !used.contains(t))
            .map(|(style, variant)| {
                let components = required_components(Some(l), sock, style, art, variant);
                let completeness = completeness(&components);
                SpareKit { style, variant, components, completeness }
            })
            .collect(),
        _ => Vec::new(),
    };

    let mut notes = Vec::new();
    if matches!(contract, KitContract::Measured { .. }) {
        notes.push(if nameplates {
            "Nameplate: `jersey/name_<style>_<art id>_<variant>_cm` — the row's own triple names the sheet on every row of X360 / PS3 Legacy, X360 / PS3 NHL 15 and PS3 NHL 12 / 13 (research R101 / R103). A naming association, not the game's runtime selection; not part of the kit's completeness.".to_string()
        } else {
            "Nameplate: the `name_<style>_<art id>_<variant>_cm` naming is measured on X360 / PS3 Legacy, X360 / PS3 NHL 15 and PS3 NHL 12 / 13 only (research R103); it is not looked up on this title.".to_string()
        });
    }
    match &contract {
        KitContract::Measured { .. } => {}
        KitContract::NoSelectorTable { reason } | KitContract::Unmeasured { reason } => notes.push(reason.clone()),
    }
    if rendering_root.is_none() {
        notes.push("no rendering tree is attached, so the kits are listed without their textures and none is called complete".to_string());
    }
    if let Some(l) = &listing {
        for (part, evidence) in l.directories() {
            match evidence {
                DirEvidence::Listed { .. } => {}
                DirEvidence::Absent => notes.push(format!("the `{}/` directory is absent: every {} component is missing", part.dir(), part.as_str())),
                DirEvidence::Unreadable { reason } => notes.push(format!("the `{}/` directory could not be read ({reason}): its components are unknown, not proven absent", part.dir())),
                DirEvidence::Archived { archive, files } => notes.push(format!(
                    "the `{}/` directory is not loose: {files} {} member{} were listed from the render archive ({archive}) by file table; presence is archive-listing evidence; a thumbnail is decoded from the member on demand and nothing is written to the archive",
                    part.dir(),
                    part.as_str(),
                    if files == 1 { "" } else { "s" }
                )),
            }
        }
        if let (Some(art), Some(sock)) = (team_art_id, sock) {
            let other = l.sock_stems_in_other_grammar(art, sock);
            if other > 0 {
                notes.push(format!(
                    "{other} sock file{} under this art id use the other measured grammar ({}); the contract for this title is {}, so they are observed but not joined",
                    if other == 1 { "" } else { "s" },
                    match sock {
                        SockGrammar::SingleFile => "two-file",
                        SockGrammar::TwoFile => "single-file",
                    },
                    match sock {
                        SockGrammar::SingleFile => "single-file",
                        SockGrammar::TwoFile => "two-file",
                    }
                ));
            }
        }
        let count = |c: Completeness| kits.iter().filter(|k| k.refusal.is_none() && k.completeness == c).count();
        let spare_count = |c: Completeness| spares.iter().filter(|s| s.completeness == c).count();
        notes.push(format!(
            "{} kit row{} on this team: {} complete, {} incomplete, {} unknown, {} refused; {} shipped spare candidate{} no row points at: {} complete, {} incomplete, {} unknown",
            kits.len(),
            if kits.len() == 1 { "" } else { "s" },
            count(Completeness::Complete),
            count(Completeness::Incomplete),
            count(Completeness::Unknown),
            kits.iter().filter(|k| k.refusal.is_some()).count(),
            spares.len(),
            if spares.len() == 1 { "" } else { "s" },
            spare_count(Completeness::Complete),
            spare_count(Completeness::Incomplete),
            spare_count(Completeness::Unknown),
        ));
    }
    notes.push(
        "the join is `(style, team art id, variant)` → jersey / pant / sock, measured at 100 % of rows on NHL 11–Legacy (research 2026-09-21 pm handoff R56); which kit the game dresses a team in for a given game is runtime and is not claimed"
            .to_string(),
    );
    let reassignment_reason = match sock {
        Some(SockGrammar::TwoFile) => "re-pointing a row has no write contract yet; on this title the sock 0 / 1 sub-index is a mirrored pair of one kit (research R74), so any future re-point must move both files together and no 0 / 1 selector is offered".to_string(),
        Some(SockGrammar::SingleFile) => "re-pointing a row has no write contract yet (validation against the complete spare candidates, the pending-review path and a runtime card are all still to be written)".to_string(),
        None => "the kit-file contract is not measured for this title, so nothing could be re-pointed".to_string(),
    };
    TeamKitsView {
        team_id,
        team_art_id,
        contract,
        rendering_root: rendering_root.map(|r| r.to_string_lossy().into_owned()),
        directories: listing.as_ref().map(|l| l.directories()).unwrap_or_default(),
        kits,
        spares,
        reassignment_supported: false,
        reassignment_reason,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team_tabs::TeamJersey;

    /// A synthetic `EB\0\3` v`0x00000400` archive (the PS3 NHL 12 / 13 / Xbox 360 Legacy form:
    /// 16-byte file-table entries) holding `members` as `(dir, basename)` with 16-byte payloads.
    fn write_eb_v400(path: &Path, dirs: &[&str], members: &[(u16, &str)]) {
        const HDR: usize = 0x30;
        let align16 = |n: usize| (n + 15) & !15;
        let name_slot = members.iter().map(|(_, n)| n.len() + 3).max().unwrap_or(3);
        let dir_slot = dirs.iter().map(|d| d.len() + 1).max().unwrap_or(1);
        let name_off = HDR + members.len() * 16;
        let dir_start = align16(name_off + members.len() * name_slot);
        let data_start = align16(dir_start + dirs.len() * dir_slot);
        let total = data_start + members.len() * 16;
        let mut out = vec![0u8; total];
        out[0..4].copy_from_slice(b"EB\x00\x03");
        out[0x04..0x08].copy_from_slice(&(members.len() as u32).to_be_bytes());
        out[0x08..0x0C].copy_from_slice(&0x0000_0400u32.to_be_bytes());
        out[0x0C..0x10].copy_from_slice(&(name_off as u32).to_be_bytes());
        out[0x14] = name_slot as u8;
        out[0x15] = dir_slot as u8;
        out[0x16..0x18].copy_from_slice(&(dirs.len() as u16).to_be_bytes());
        out[0x18..0x20].copy_from_slice(&(total as u64).to_be_bytes());
        for (i, (di, name)) in members.iter().enumerate() {
            let e = HDR + i * 16;
            out[e..e + 4].copy_from_slice(&(((data_start + i * 16) >> 4) as u32).to_be_bytes());
            out[e + 8..e + 12].copy_from_slice(&16u32.to_be_bytes());
            let n = name_off + i * name_slot;
            out[n..n + 2].copy_from_slice(&di.to_be_bytes());
            out[n + 2..n + 2 + name.len()].copy_from_slice(name.as_bytes());
        }
        for (i, d) in dirs.iter().enumerate() {
            let o = dir_start + i * dir_slot;
            out[o..o + d.len()].copy_from_slice(d.as_bytes());
        }
        std::fs::write(path, out).unwrap();
    }

    /// Coordinator review of receipts 10–17, item 3 (22 Sep 2026): PS3 NHL 12 / 13 keep their
    /// kits inside `cacherender.big` / `nocacherender.big`. With no loose `jersey/` / `pant/` /
    /// `sock/` directories, the members are listed from the archives' file tables under the same
    /// contract rule: both sock files stay required on the two-file grammar, a member present in
    /// either archive is present (with its archive named), a member in neither is missing, and a
    /// loose directory — when one exists — keeps its loose evidence. An archive that does not
    /// parse makes the part unknown, never absent; no archive at all leaves it absent.
    #[test]
    fn ps3_archive_backed_kits_are_listed_from_the_render_archives_under_the_same_rule() {
        let usrdir = std::env::temp_dir().join(format!("nhlms-kit-eb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&usrdir);
        let rendering = usrdir.join("rendering");
        std::fs::create_dir_all(rendering.join("boot")).unwrap();
        write_eb_v400(
            &usrdir.join("cacherender.big"),
            &["rendering/jersey", "rendering/pant", "rendering/sock"],
            &[(0, "texlib_0_7_0.rpsgl"), (1, "texlib_0_7_0.rpsgl"), (2, "sock_0_0_7_0_cm.rpsgl"), (0, "texlib_1_7_1.rpsgl"), (1, "texlib_1_7_1.rpsgl")],
        );
        write_eb_v400(&usrdir.join("nocacherender.big"), &["rendering/sock", "rendering/jersey"], &[(0, "sock_0_1_7_0_cm.rpsgl"), (1, "texlib_2_7_0.rpsgl")]);
        let contract = kit_contract(Some("nhl12"), Platform::Ps3);
        let rows = vec![row(0, 0, 7, 0), row(1, 1, 7, 1)];
        let v = team_kits(7, Some(7), &rows, contract, Some(&rendering), false);
        for (part, e) in &v.directories {
            match e {
                DirEvidence::Archived { archive, files } => {
                    assert!(archive.contains("cacherender.big") && archive.contains("nocacherender.big"), "{part:?}: {archive}");
                    assert_eq!(*files, match part { KitPart::Jersey => 3, KitPart::Pant => 2, KitPart::Sock => 2 }, "{part:?}");
                }
                other => panic!("{part:?}: expected archive evidence, got {other:?}"),
            }
        }
        // Row (0, 7, 0): jersey + pants + both socks, across the two archives — complete.
        let k0 = &v.kits[0];
        assert_eq!(k0.completeness, Completeness::Complete, "{:?}", k0.components);
        assert_eq!(k0.components.len(), 4);
        assert!(k0.components.iter().all(|c| c.status == ComponentStatus::Present && c.path.is_none()));
        assert_eq!(k0.components[0].archive_member.as_deref(), Some("cacherender.big:rendering/jersey/texlib_0_7_0.rpsgl"));
        assert_eq!((k0.components[0].archive.as_deref(), k0.components[0].member_path.as_deref()), (Some("cacherender.big"), Some("rendering/jersey/texlib_0_7_0.rpsgl")));
        assert_eq!(k0.components[3].archive_member.as_deref(), Some("nocacherender.big:rendering/sock/sock_0_1_7_0_cm.rpsgl"));
        // Row (1, 7, 1): jersey + pants present, both sock files missing — incomplete, with the
        // archive named in the reason.
        let k1 = &v.kits[1];
        assert_eq!(k1.completeness, Completeness::Incomplete);
        let socks: Vec<&KitComponent> = k1.components.iter().filter(|c| c.part == KitPart::Sock).collect();
        assert_eq!(socks.len(), 2);
        assert!(socks.iter().all(|c| c.status == ComponentStatus::Missing && c.reason.as_deref().unwrap().contains("not among the members of")), "{socks:?}");
        // The spare (2, 7, 0): jersey only — incomplete, never a complete spare.
        assert_eq!(v.spares.len(), 1);
        assert_eq!((v.spares[0].style, v.spares[0].variant, v.spares[0].completeness), (2, 0, Completeness::Incomplete));
        assert!(v.notes.iter().any(|n| n.contains("not loose") && n.contains("decoded from the member on demand")), "{:?}", v.notes);

        // A loose directory beside the archives keeps its own evidence; only absent parts consult
        // the archives.
        std::fs::create_dir_all(rendering.join("pant")).unwrap();
        std::fs::write(rendering.join("pant/texlib_0_7_0.rpsgl"), b"x").unwrap();
        let v = team_kits(7, Some(7), &rows, kit_contract(Some("nhl12"), Platform::Ps3), Some(&rendering), false);
        let (_, pant) = v.directories.iter().find(|(p, _)| *p == KitPart::Pant).unwrap();
        assert!(matches!(pant, DirEvidence::Listed { files: 1 }), "{pant:?}");
        assert!(v.kits[0].components[1].path.is_some() && v.kits[0].components[1].archive_member.is_none());
        assert_eq!(v.kits[0].completeness, Completeness::Complete);
        assert_eq!(v.kits[1].components[1].status, ComponentStatus::Missing, "the loose pant dir lacks (1, 7, 1); the archive is not consulted for a loose part");

        // An archive that does not parse: unknown, not absent.
        std::fs::write(usrdir.join("nocacherender.big"), b"EB\x00\x03garbage").unwrap();
        let v = team_kits(7, Some(7), &rows, kit_contract(Some("nhl12"), Platform::Ps3), Some(&rendering), false);
        let (_, sock) = v.directories.iter().find(|(p, _)| *p == KitPart::Sock).unwrap();
        assert!(matches!(sock, DirEvidence::Unreadable { .. }), "{sock:?}");
        assert_eq!(v.kits[0].completeness, Completeness::Unknown);

        // No archives at all: absent, as before.
        std::fs::remove_file(usrdir.join("cacherender.big")).unwrap();
        std::fs::remove_file(usrdir.join("nocacherender.big")).unwrap();
        let v = team_kits(7, Some(7), &rows, kit_contract(Some("nhl12"), Platform::Ps3), Some(&rendering), false);
        let (_, sock) = v.directories.iter().find(|(p, _)| *p == KitPart::Sock).unwrap();
        assert!(matches!(sock, DirEvidence::Absent), "{sock:?}");
        let _ = std::fs::remove_dir_all(&usrdir);
    }

    fn row(record: u32, style: u32, art: u32, variant: u32) -> TeamJersey {
        TeamJersey {
            record,
            jersey_index: record,
            style_raw: style,
            team_art_id: art,
            variant_raw: variant,
            fe_variant_raw: variant,
            helmet_logo: 1,
            helmet_numbers: 0,
            is_light_color: 1,
            has_tie: 0,
            loc_string_raw: 0,
            is_default: record == 0,
            is_lockable: false,
        }
    }

    fn single() -> KitContract {
        kit_contract(Some("nhl15"), Platform::Xbox)
    }
    fn two_file() -> KitContract {
        kit_contract(Some("nhl11"), Platform::Xbox)
    }

    /// A disposable directory that removes itself, matching the crate's convention (no `tempfile`
    /// dev-dependency; see `discovery.rs`).
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("nhlms-kit-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
        fn path(&self) -> &Path {
            &self.0
        }
        fn put(&self, part: KitPart, s: u64, a: u64, v: u64, sub: Option<u64>) {
            let dir = self.0.join(part.dir());
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(format!("{}.rx2", part.file_stem(s, a, v, sub))), b"x").unwrap();
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn statuses(k: &[KitComponent]) -> Vec<(&str, ComponentStatus)> {
        k.iter().map(|c| (c.label.as_str(), c.status)).collect()
    }

    // --- complete supported layouts -------------------------------------------------------

    #[test]
    fn a_complete_single_file_kit_resolves_every_component_and_the_spare_pool_uses_the_same_rule() {
        let d = Tmp::new("single");
        for (s, v) in [(0u64, 0u64), (1, 3), (1, 9)] {
            for p in [KitPart::Jersey, KitPart::Pant, KitPart::Sock] {
                d.put(p, s, 7, v, None);
            }
        }
        d.put(KitPart::Jersey, 0, 7, 5, None); // jersey only — an incomplete spare, listed as such
        let rows = [row(0, 0, 7, 0), row(1, 1, 7, 3)];
        let v = team_kits(42, Some(7), &rows, single(), Some(d.path()), false);
        assert_eq!(v.kits.len(), 2);
        assert!(v.kits.iter().all(|k| k.completeness == Completeness::Complete && k.refusal.is_none()));
        assert_eq!(statuses(&v.kits[0].components), [("jersey", ComponentStatus::Present), ("pants", ComponentStatus::Present), ("socks", ComponentStatus::Present)]);
        assert!(v.kits[0].components[2].path.as_ref().unwrap().contains("sock_0_7_0_cm"), "the sock template carries the _cm suffix");
        assert!(v.kits[0].is_default && !v.kits[1].is_default);

        // The spare pool lists both unused triples, each with its own verdict from the shared rule.
        let by_triple: BTreeMap<(u64, u64), Completeness> = v.spares.iter().map(|s| ((s.style, s.variant), s.completeness)).collect();
        assert_eq!(by_triple.get(&(1, 9)), Some(&Completeness::Complete));
        assert_eq!(by_triple.get(&(0, 5)), Some(&Completeness::Incomplete));
        let partial = v.spares.iter().find(|s| (s.style, s.variant) == (0, 5)).unwrap();
        assert_eq!(statuses(&partial.components), [("jersey", ComponentStatus::Present), ("pants", ComponentStatus::Missing), ("socks", ComponentStatus::Missing)]);
        assert!(partial.components[1].reason.as_deref().unwrap().contains("not in the listed directory"));
        assert!(v.notes.iter().any(|n| n.contains("2 shipped spare candidates no row points at: 1 complete, 1 incomplete, 0 unknown")), "{:?}", v.notes);
        assert!(!v.reassignment_supported);
    }

    /// R101 / R103 (26 Sep): the nameplate sheet of a row's own triple is looked up beside the kit on
    /// measured titles only, and never changes the kit's completeness; `name_*` never becomes a spare.
    #[test]
    fn the_nameplate_sheet_is_an_association_beside_the_kit_not_a_required_component() {
        assert!(nameplate_measured(Some("nhllegacy"), Platform::Xbox) && nameplate_measured(Some("nhl15"), Platform::Ps3));
        assert!(nameplate_measured(Some("nhl12"), Platform::Ps3) && nameplate_measured(Some("nhl13"), Platform::Ps3));
        assert!(!nameplate_measured(Some("nhl12"), Platform::Xbox), "X360 NHL 12 is not in R103");
        assert!(!nameplate_measured(Some("nhl14"), Platform::Ps3) && !nameplate_measured(Some("nhl11"), Platform::Xbox) && !nameplate_measured(None, Platform::Ps3));
        assert!(!nameplate_measured(Some("nhllegacy-recompiled"), Platform::Recomp), "Recompiled not measured");
        let d = Tmp::new("nameplate");
        for v in [0u64, 3] {
            for p in [KitPart::Jersey, KitPart::Pant, KitPart::Sock] {
                d.put(p, 0, 7, v, None);
            }
        }
        std::fs::write(d.path().join("jersey").join("name_0_7_0_cm.rx2"), b"x").unwrap();
        std::fs::write(d.path().join("jersey").join("name_1_7_9_cm.rx2"), b"x").unwrap(); // a spare name: no row
        let rows = [row(0, 0, 7, 0), row(1, 0, 7, 3)];
        let v = team_kits(42, Some(7), &rows, single(), Some(d.path()), true);
        let np: Vec<_> = v.kits.iter().map(|k| k.nameplate.as_ref().map(|c| (c.file_stem.clone(), c.status))).collect();
        assert_eq!(np, [Some(("name_0_7_0_cm".to_string(), ComponentStatus::Present)), Some(("name_0_7_3_cm".to_string(), ComponentStatus::Missing))]);
        assert!(v.kits.iter().all(|k| k.completeness == Completeness::Complete), "a missing nameplate does not make the kit incomplete");
        assert!(v.spares.is_empty(), "name_* sheets are not kit triples: {:?}", v.spares);
        assert!(v.notes.iter().any(|n| n.contains("naming association") && n.contains("R103")), "{:?}", v.notes);
        // Unmeasured title: no lookup, and the note says so.
        let v = team_kits(42, Some(7), &rows, single(), Some(d.path()), false);
        assert!(v.kits.iter().all(|k| k.nameplate.is_none()));
        assert!(v.notes.iter().any(|n| n.contains("not looked up on this title")), "{:?}", v.notes);
    }

    #[test]
    fn a_complete_two_file_kit_lists_both_sock_files_as_separate_components() {
        let d = Tmp::new("twofile");
        d.put(KitPart::Jersey, 0, 7, 0, None);
        d.put(KitPart::Pant, 0, 7, 0, None);
        d.put(KitPart::Sock, 0, 7, 0, Some(0));
        d.put(KitPart::Sock, 0, 7, 0, Some(1));
        let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], two_file(), Some(d.path()), false);
        assert_eq!(v.kits[0].completeness, Completeness::Complete);
        assert_eq!(
            statuses(&v.kits[0].components),
            [("jersey", ComponentStatus::Present), ("pants", ComponentStatus::Present), ("socks (file 0)", ComponentStatus::Present), ("socks (file 1)", ComponentStatus::Present)]
        );
        assert!(v.kits[0].components[2].path.as_ref().unwrap().contains("sock_0_0_7_0_cm"));
        assert!(v.kits[0].components[3].path.as_ref().unwrap().contains("sock_0_1_7_0_cm"));
        assert!(v.reassignment_reason.contains("sub-index") && v.reassignment_reason.contains("mirrored pair"), "{}", v.reassignment_reason);
    }

    // --- the F1 regression: no sock directory / empty / partial ------------------------------

    #[test]
    fn f1_no_sock_directory_never_yields_a_complete_kit_or_spare() {
        // The coordinator's reproduction: jersey + pant only, no `sock/` at all.
        let d = Tmp::new("nosockdir");
        d.put(KitPart::Jersey, 0, 7, 0, None);
        d.put(KitPart::Pant, 0, 7, 0, None);
        d.put(KitPart::Jersey, 1, 7, 2, None); // a would-be spare
        d.put(KitPart::Pant, 1, 7, 2, None);
        for contract in [single(), two_file()] {
            let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], contract, Some(d.path()), false);
            let k = &v.kits[0];
            assert_eq!(k.completeness, Completeness::Incomplete, "{:?}", k);
            let socks: Vec<&KitComponent> = k.components.iter().filter(|c| c.part == KitPart::Sock).collect();
            assert!(!socks.is_empty(), "the sock components must still be listed");
            assert!(socks.iter().all(|c| c.status == ComponentStatus::Missing && c.reason.as_deref().unwrap().contains("absent")));
            // Jersey and pants stay visible as present — partial information is kept.
            assert_eq!(k.components[0].status, ComponentStatus::Present);
            assert_eq!(k.components[1].status, ComponentStatus::Present);
            // The spare is listed, and it is *incomplete* — never a complete spare.
            assert_eq!(v.spares.len(), 1);
            assert_eq!(v.spares[0].completeness, Completeness::Incomplete);
            assert!(v.directories.iter().any(|(p, e)| *p == KitPart::Sock && *e == DirEvidence::Absent));
            assert!(v.notes.iter().any(|n| n.contains("`sock/` directory is absent")), "{:?}", v.notes);
        }
    }

    #[test]
    fn f1_an_empty_sock_directory_is_missing_evidence_not_complete() {
        let d = Tmp::new("emptysock");
        d.put(KitPart::Jersey, 0, 7, 0, None);
        d.put(KitPart::Pant, 0, 7, 0, None);
        std::fs::create_dir_all(d.path().join("sock")).unwrap();
        let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], single(), Some(d.path()), false);
        assert_eq!(v.kits[0].completeness, Completeness::Incomplete);
        assert_eq!(v.kits[0].components[2].status, ComponentStatus::Missing);
        assert!(v.kits[0].components[2].reason.as_deref().unwrap().contains("not in the listed directory"));
        assert!(v.directories.iter().any(|(p, e)| *p == KitPart::Sock && *e == DirEvidence::Listed { files: 0 }));
    }

    #[test]
    fn f1_the_two_file_form_with_one_sock_file_is_incomplete_and_names_the_missing_file() {
        let d = Tmp::new("halfsock");
        d.put(KitPart::Jersey, 0, 7, 0, None);
        d.put(KitPart::Pant, 0, 7, 0, None);
        d.put(KitPart::Sock, 0, 7, 0, Some(0)); // only sub-index 0
        let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], two_file(), Some(d.path()), false);
        let k = &v.kits[0];
        assert_eq!(k.completeness, Completeness::Incomplete);
        assert_eq!(
            statuses(&k.components),
            [("jersey", ComponentStatus::Present), ("pants", ComponentStatus::Present), ("socks (file 0)", ComponentStatus::Present), ("socks (file 1)", ComponentStatus::Missing)]
        );
        assert_eq!(k.components[3].file_stem, "sock_0_1_7_0_cm");

        // The same half-kit as a spare (no row) gets the same verdict.
        let v = team_kits(42, Some(7), &[], two_file(), Some(d.path()), false);
        assert_eq!(v.spares.len(), 1);
        assert_eq!(v.spares[0].completeness, Completeness::Incomplete);
        assert_eq!(v.spares[0].components[3].status, ComponentStatus::Missing);
    }

    #[test]
    fn f1_files_in_the_other_sock_grammar_are_observed_but_not_joined() {
        // A single-file title whose tree carries only two-file sock names: the contract decides,
        // the socks are missing, and the note says what was seen.
        let d = Tmp::new("othergrammar");
        d.put(KitPart::Jersey, 0, 7, 0, None);
        d.put(KitPart::Pant, 0, 7, 0, None);
        d.put(KitPart::Sock, 0, 7, 0, Some(0));
        d.put(KitPart::Sock, 0, 7, 0, Some(1));
        let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], single(), Some(d.path()), false);
        assert_eq!(v.kits[0].completeness, Completeness::Incomplete);
        assert_eq!(v.kits[0].components[2].status, ComponentStatus::Missing);
        assert!(v.notes.iter().any(|n| n.contains("2 sock files under this art id use the other measured grammar")), "{:?}", v.notes);
        // And the reverse: a two-file title with single-file names.
        let d = Tmp::new("othergrammar2");
        d.put(KitPart::Jersey, 0, 7, 0, None);
        d.put(KitPart::Pant, 0, 7, 0, None);
        d.put(KitPart::Sock, 0, 7, 0, None);
        let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], two_file(), Some(d.path()), false);
        assert_eq!(v.kits[0].completeness, Completeness::Incomplete);
        assert!(v.notes.iter().any(|n| n.contains("1 sock file under this art id use the other measured grammar")), "{:?}", v.notes);
    }

    // --- unknown evidence: unreadable directory, no tree, unmeasured contract ----------------

    #[test]
    fn f1_an_unreadable_sock_directory_is_unknown_not_absent() {
        // `sock` exists but is a regular file: `read_dir` fails with something other than NotFound.
        let d = Tmp::new("unreadable");
        d.put(KitPart::Jersey, 0, 7, 0, None);
        d.put(KitPart::Pant, 0, 7, 0, None);
        std::fs::write(d.path().join("sock"), b"not a directory").unwrap();
        let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], single(), Some(d.path()), false);
        let k = &v.kits[0];
        assert_eq!(k.completeness, Completeness::Unknown, "{:?}", k);
        assert_eq!(k.components[2].status, ComponentStatus::Unknown);
        assert!(k.components[2].reason.as_deref().unwrap().contains("could not be read"));
        assert!(matches!(v.directories.iter().find(|(p, _)| *p == KitPart::Sock).unwrap().1, DirEvidence::Unreadable { .. }));
        // A spare triple named by the jersey directory is unknown too — not offered as complete.
        d.put(KitPart::Jersey, 1, 7, 4, None);
        d.put(KitPart::Pant, 1, 7, 4, None);
        let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], single(), Some(d.path()), false);
        assert_eq!(v.spares.len(), 1);
        assert_eq!(v.spares[0].completeness, Completeness::Unknown);
        assert!(v.notes.iter().any(|n| n.contains("unknown, not proven absent")), "{:?}", v.notes);
    }

    #[test]
    fn a_missing_component_beats_an_unknown_one_in_the_verdict() {
        // Pants absent (definite) and socks unreadable (unknown): the kit is incomplete.
        let d = Tmp::new("mixed");
        d.put(KitPart::Jersey, 0, 7, 0, None);
        std::fs::write(d.path().join("sock"), b"not a directory").unwrap();
        let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], single(), Some(d.path()), false);
        assert_eq!(v.kits[0].completeness, Completeness::Incomplete);
        assert_eq!(statuses(&v.kits[0].components), [("jersey", ComponentStatus::Present), ("pants", ComponentStatus::Missing), ("socks", ComponentStatus::Unknown)]);
    }

    #[test]
    fn without_a_rendering_tree_the_kits_list_but_claim_nothing() {
        let rows = [row(0, 0, 7, 0)];
        let v = team_kits(42, Some(7), &rows, single(), None, false);
        assert_eq!(v.kits.len(), 1);
        assert_eq!(v.kits[0].completeness, Completeness::Unknown);
        assert_eq!(v.kits[0].components.len(), 3);
        assert!(v.kits[0].components.iter().all(|c| c.status == ComponentStatus::Unknown && c.path.is_none()));
        assert!(v.spares.is_empty());
        assert!(v.directories.is_empty());
        assert!(v.notes.iter().any(|n| n.contains("no rendering tree")));
    }

    #[test]
    fn an_unmeasured_or_tableless_title_lists_rows_with_no_components_and_no_spares() {
        let d = Tmp::new("unmeasured");
        for p in [KitPart::Jersey, KitPart::Pant, KitPart::Sock] {
            d.put(p, 0, 7, 0, None);
            d.put(p, 1, 7, 1, None);
        }
        for (game, platform, marker) in [
            (Some("nhl08"), Platform::Xbox, "not measured"),
            (None, Platform::Xbox, "no recorded title"),
            (Some("nhl21"), Platform::Ps4, "PS4"),
            (Some("nhl10"), Platform::Xbox, "no `stockteamjerseys` table"),
        ] {
            let contract = kit_contract(game, platform);
            assert!(!matches!(contract, KitContract::Measured { .. }), "{game:?} / {platform:?}");
            let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], contract, Some(d.path()), false);
            assert_eq!(v.kits[0].completeness, Completeness::Unknown, "{game:?}");
            assert!(v.kits[0].components.is_empty());
            assert!(v.spares.is_empty(), "no spare is offered without a measured contract even though files exist");
            assert!(v.notes.iter().any(|n| n.contains(marker)), "{game:?}: {:?}", v.notes);
            assert!(!v.reassignment_supported);
        }
    }

    #[test]
    fn the_contract_follows_the_title_and_platform_not_the_tree() {
        let measured = |c: KitContract| match c {
            KitContract::Measured { sock, extension, .. } => (sock, extension),
            other => panic!("expected a measured contract, got {other:?}"),
        };
        assert_eq!(measured(kit_contract(Some("nhl11"), Platform::Xbox)), (SockGrammar::TwoFile, "rx2".to_string()));
        assert_eq!(measured(kit_contract(Some("nhl12"), Platform::Ps3)), (SockGrammar::TwoFile, "rpsgl".to_string()));
        assert_eq!(measured(kit_contract(Some("nhl13"), Platform::Xbox)).0, SockGrammar::SingleFile);
        assert_eq!(measured(kit_contract(Some("nhllegacy"), Platform::Recomp)), (SockGrammar::SingleFile, "rx2".to_string()));
        assert!(matches!(kit_contract(Some("nhl09"), Platform::Ps3), KitContract::NoSelectorTable { .. }));
        assert!(matches!(kit_contract(Some("nhl15"), Platform::Ps4), KitContract::Unmeasured { .. }));
    }

    // --- refusals and parsing ----------------------------------------------------------------

    #[test]
    fn custom_team_rows_and_art_id_mismatches_are_refused_with_their_reason() {
        let d = Tmp::new("refuse");
        for p in [KitPart::Jersey, KitPart::Pant, KitPart::Sock] {
            d.put(p, 0, 7, 0, None);
        }
        let custom = [row(0, 0, CUSTOM_TEAM_ART_ID as u32, 0)];
        let v = team_kits(1, Some(CUSTOM_TEAM_ART_ID), &custom, single(), Some(d.path()), false);
        assert!(v.kits[0].refusal.as_deref().is_some_and(|r| r.contains("20054")), "{:?}", v.kits[0].refusal);
        assert!(v.kits[0].components.is_empty() && v.kits[0].completeness == Completeness::Incomplete);
        assert!(v.spares.is_empty(), "the custom art id has no spare pool");

        let wrong = [row(0, 0, 9, 0)];
        let v = team_kits(42, Some(7), &wrong, single(), Some(d.path()), false);
        assert!(v.kits[0].refusal.as_deref().is_some_and(|r| r.contains("not the team's own art id")), "{:?}", v.kits[0].refusal);
    }

    #[test]
    fn stem_parsing_is_the_exact_inverse_of_the_templates() {
        assert_eq!(KitPart::Jersey.parse_stem(SockGrammar::SingleFile, "texlib_1_7_3"), Some((1, 7, 3, None)));
        assert_eq!(KitPart::Sock.parse_stem(SockGrammar::SingleFile, "sock_1_7_3_cm"), Some((1, 7, 3, None)));
        assert_eq!(KitPart::Sock.parse_stem(SockGrammar::TwoFile, "sock_1_0_7_3_cm"), Some((1, 7, 3, Some(0))));
        // Wrong grammar, wrong family, sibling families, leading zeros, a sub-index above 1.
        assert_eq!(KitPart::Sock.parse_stem(SockGrammar::TwoFile, "sock_1_7_3_cm"), None);
        assert_eq!(KitPart::Sock.parse_stem(SockGrammar::SingleFile, "sock_1_0_7_3_cm"), None);
        assert_eq!(KitPart::Jersey.parse_stem(SockGrammar::SingleFile, "name_1_7_3_cm"), None);
        assert_eq!(KitPart::Sock.parse_stem(SockGrammar::SingleFile, "playersock_0_0_am"), None);
        assert_eq!(KitPart::Sock.parse_stem(SockGrammar::SingleFile, "sock_1_7_3_nm"), None);
        assert_eq!(KitPart::Jersey.parse_stem(SockGrammar::SingleFile, "texlib_01_7_3"), None);
        assert_eq!(KitPart::Sock.parse_stem(SockGrammar::TwoFile, "sock_1_2_7_3_cm"), None);
    }

    #[test]
    fn a_ps3_tree_resolves_rpsgl_and_a_mixed_tree_prefers_the_platform_extension() {
        let d = Tmp::new("ps3");
        for p in [KitPart::Jersey, KitPart::Pant, KitPart::Sock] {
            let dir = d.path().join(p.dir());
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(format!("{}.rpsgl", p.file_stem(0, 7, 0, None))), b"x").unwrap();
        }
        // A stray .rx2 twin of the jersey: the PS3 contract still picks .rpsgl.
        std::fs::write(d.path().join("jersey").join("texlib_0_7_0.rx2"), b"x").unwrap();
        let v = team_kits(42, Some(7), &[row(0, 0, 7, 0)], kit_contract(Some("nhl15"), Platform::Ps3), Some(d.path()), false);
        assert_eq!(v.kits[0].completeness, Completeness::Complete);
        assert!(v.kits[0].components.iter().all(|c| c.path.as_ref().unwrap().ends_with(".rpsgl")), "{:?}", v.kits[0].components);
    }
}
