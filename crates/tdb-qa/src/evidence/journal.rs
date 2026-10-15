//! Persisted-but-unsealed emitter journals — **incomplete execution evidence**.
//!
//! A journal is what an emitter writes *while it runs*: identity first, then every declaration and
//! every observation as it happens. It lives at `<root>/auto/in-progress/<journal_id>/`, deliberately
//! outside `auto/runs/`, because it is not a run: it has no terminal state and none is invented for
//! it. Sealing turns a journal into an immutable run; nothing else may.
//!
//! # Why append-only
//!
//! The first implementation rewrote the whole observations file on every record. A write that is
//! interrupted part-way through that rewrite destroys **every** previously persisted observation —
//! exactly the evidence the journal exists to protect. Every record file here is therefore
//! **append-only**: a partial write can only damage the record being appended, and the loader drops
//! that fragment and keeps everything before it intact.
//!
//! # Record framing (`qa-run-journal/3`)
//!
//! Two properties make a record unambiguous, and both had to be fixed in R2:
//!
//! * **One record is exactly one physical line.** Field values are escaped before they are written
//!   ([`escape_field`]), so no raw newline, carriage return or other control character ever reaches
//!   the file. A CSV field may legally contain a newline, so without this a record could span
//!   several lines and "the text after the last newline is a truncated tail" would be wrong — an
//!   interruption inside a quoted multi-line field would look like a complete record.
//! * **The digest is length-prefixed, not separator-delimited.** `/2` hashed the fields joined by
//!   `0x1f` and claimed that byte "cannot occur in a CSV field value", which is false: a quoted CSV
//!   field may contain any byte. `["a\x1fb"]` and `["a", "b"]` hashed identically. [`record_digest`]
//!   now folds in the field count and each field's byte length before its bytes, so no two distinct
//!   field lists can collide.
//!
//! The digest is taken over the **unescaped** values, so it certifies what was meant, not how it
//! was spelled. Because those semantics changed, the format is versioned: a `/2` journal is refused
//! rather than reinterpreted under rules it was not written to.
//!
//! # Integrity, and what "fail closed" means here
//!
//! | Damage | Handling |
//! |---|---|
//! | a record file that is absent | not damage — the emitter had not written one yet |
//! | a record file that exists but **cannot be read** — permission denied, invalid UTF-8, a directory or other non-regular entry at its name | **hard error.** An unreadable file is not an empty file, and reporting it as "no records" would silently convert evidence into absence |
//! | trailing record with no terminating newline | the append was interrupted — that record is dropped, everything before it is kept, and `truncated-trailing-record` is reported |
//! | a record whose digest does not match, or whose escaping is malformed | that record is dropped and `corrupt-record:<file>:<line>` is reported; neighbours are unaffected because each record is verified on its own |
//! | a record that parses but whose schema, key or outcome does not | dropped, reported |
//! | **damaged declarations** — `keys-declared.txt` truncated, unparseable or unreadable | **hard error.** Declarations bound the blast radius of every other kind of damage (below); without them nothing can be bounded |
//! | `journal.json` absent, unreadable, unparseable, wrong schema, or naming a different id than its directory | **hard error** — identity cannot be established, so no material may be attributed to anything |
//!
//! **Dropped is not forgiven.** A journal carrying any [`Journal::integrity`] finding is *damaged*,
//! and the merge withholds qualification across **every key that journal declared** — the
//! conservatively affected scope. A damaged record's own key cannot be trusted (that is what
//! "damaged" means), so it is never guessed at; the declared set is the honest bound, and it is why
//! damaged declarations are fatal rather than merely reported. A journal with damage but no
//! declarations at all is refused for the same reason.
//!
//! This is deliberately different from an *undamaged* active journal: a clean incomplete pass
//! qualifies nothing on its own, but it does not invalidate an older completed run either. Only
//! uncertainty does.
//!
//! Nothing is ever repaired in place: this module only reads. Damaged material stays on disk for a
//! human to look at, and every drop is reported.
//!
//! # Double counting
//!
//! A successful seal writes `sealed_run_id` into `journal.json` before removing the directory. If
//! removal fails, or the process dies between the two, the journal survives — and [`load_journals`]
//! may then skip it, because the run it names holds the same evidence.
//!
//! **A matching id is not enough.** R2 verifies the link before deduplicating: the run must agree
//! with the journal's identity (emitter, start, scope, digests) *and* contain every verified
//! declaration, observation, unmapped record and facet the journal holds. Anything else — a
//! `sealed_run_id` pointing at an unrelated run, or a run missing records the journal proves were
//! made — leaves the journal loaded and flagged, and therefore non-qualifying, rather than
//! discarding evidence on the strength of a string match.
//!
//! **Nor is agreement of the surviving records** (C0-S2-INT1-R3). R2 checked identity and damage
//! independently and dropped a journal whose *verified* records all matched the named run — even
//! when other records had been dropped as corrupt or truncated. Those dropped records are precisely
//! the ones equivalence cannot examine, so the remainder says nothing about them, and discarding
//! the journal discarded its withholding effect as well: a damaged leftover silently qualified.
//! A damaged journal is therefore never deduplicated. It is retained, flagged
//! `sealed-run-unverifiable`, and keeps withholding its declared scope. Only a **clean** journal
//! proved equivalent is dropped.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::csv;
use super::key::CoverageKey;
use super::model::{Facet, Journal, Observation, Outcome, Run, UnmappedObservation};
use super::store::{self, Result, StoreError};

/// The journal directory, relative to an evidence root.
pub const DIR: &str = "in-progress";

/// Bumped from `/2` in C0-S2-INT1-R2: the digest framing and field escaping both changed, so an
/// older journal is refused rather than reinterpreted under rules it was not written to.
pub const IDENTITY_SCHEMA: &str = "qa-run-journal/3";

/// Journal record rows are **not** sealed-run rows: they carry an extra `record_sha256` column and
/// escaped values, so they have their own schema tokens.
pub const OBSERVATION_ROW: &str = "qa-journal-observation/1";
pub const UNMAPPED_ROW: &str = "qa-journal-unmapped/1";
pub const FACET_ROW: &str = "qa-journal-facet/1";

pub const OBSERVATION_HEADER: &[&str] = &[
    "schema",
    "key",
    "case_id",
    "outcome",
    "fixture_sha256",
    "output_sha256",
    "detail",
    "record_sha256",
];

pub const UNMAPPED_HEADER: &[&str] = &[
    "schema",
    "target",
    "emitter_operation",
    "case_id",
    "outcome",
    "fixture_sha256",
    "unresolved",
    "detail",
    "record_sha256",
];

pub const FACET_HEADER: &[&str] = &[
    "schema",
    "case_id",
    "key",
    "facet",
    "value",
    "record_sha256",
];

/// `journal.json`. Written **atomically** (temp + rename), so identity is either absent or whole.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityFile {
    pub schema: String,
    pub journal_id: String,
    pub emitter: String,
    pub started_utc: String,
    pub host_os: String,
    pub toolchain: String,
    pub scope_id: String,
    pub scope_version: u32,
    pub implementation_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sealed_run_id: Option<String>,
}

// ------------------------------------------------------------------ framing ------

/// Escape a field so the written record occupies exactly one physical line.
///
/// Only the backslash and the characters a line-oriented reader cannot survive are touched, so
/// ordinary text stays readable in the file.
pub fn escape_field(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// Reverse [`escape_field`]. A malformed escape makes the record uncertain, so it is an error
/// rather than a best-effort decoding.
pub fn unescape_field(value: &str) -> std::result::Result<String, String> {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('x') => {
                let hex: String = chars.by_ref().take(2).collect();
                let byte = u8::from_str_radix(&hex, 16)
                    .map_err(|_| format!("malformed \\x escape {hex:?}"))?;
                out.push(byte as char);
            }
            Some(other) => return Err(format!("unknown escape \\{other}")),
            None => return Err("trailing backslash".to_string()),
        }
    }
    Ok(out)
}

/// The digest a record carries over its own **unescaped** field values.
///
/// Length-prefixed: the field count, then each field's byte length, then its bytes. A separator
/// byte cannot do this job — a CSV field may contain any byte, including the separator — and the
/// `/2` scheme this replaces really did collide (`["a\x1fb"]` against `["a", "b"]`).
pub fn record_digest(fields: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((fields.len() as u64).to_be_bytes());
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// One appendable line: the escaped fields, then the digest of the originals.
pub fn record_line(fields: &[&str]) -> String {
    let digest = record_digest(fields);
    let mut all: Vec<String> = fields.iter().map(|f| escape_field(f)).collect();
    all.push(digest);
    csv::row(all)
}

// ------------------------------------------------------------------ reading ------

/// Read a file that is allowed to be absent but is **not** allowed to be unreadable.
///
/// `Ok(None)` means the emitter has not created it yet. Every other failure — permission denied,
/// invalid UTF-8, a link, a directory or other non-regular entry at the name — is an error, because
/// reporting an unreadable file as an empty one silently converts evidence into absence.
fn read_optional(path: &Path) -> Result<Option<String>> {
    // R5: one shared classifier for every optional evidence file — same no-follow type check, same
    // NotFound-only absence, same invalid-UTF-8 handling. Only the rule tokens stay local, so a
    // journal failure still reads as a journal failure.
    super::paths::read_optional_file(
        path,
        "journal-unexpected-entry-type",
        "journal-unreadable-file",
    )
}

/// Split a file into (complete records text, was the tail truncated).
///
/// Sound only because [`escape_field`] guarantees one record per physical line: with raw newlines
/// allowed inside quoted fields, "after the last newline" would not identify the interrupted record.
fn complete_text(raw: &str) -> (String, bool) {
    match raw.rfind('\n') {
        Some(i) => (raw[..=i].to_string(), i + 1 != raw.len()),
        None => (String::new(), !raw.is_empty()),
    }
}

/// Read a record file, verifying each record's digest independently.
fn read_records(
    path: &Path,
    header: &[&str],
    schema: &str,
    integrity: &mut Vec<String>,
) -> Result<Vec<Vec<String>>> {
    let Some(raw) = read_optional(path)? else {
        return Ok(Vec::new());
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (text, truncated) = complete_text(&raw);
    if truncated {
        integrity.push(format!("truncated-trailing-record:{name}"));
    }
    if text.trim().is_empty() {
        // A header and nothing else, or not even that. Neither is corruption.
        return Ok(Vec::new());
    }
    let records = match csv::parse_with_header(&text, header) {
        Ok(records) => records,
        Err(e) => {
            // The header itself is damaged: no record in the file can be attributed safely.
            return Err(StoreError::Rule {
                rule: "journal-record-file",
                path: path.to_path_buf(),
                detail: format!("{e}; no record in this file can be attributed"),
            });
        }
    };
    let mut out = Vec::new();
    for (i, record) in records.into_iter().enumerate() {
        let line = i + 2;
        let (escaped, digest) = record.split_at(record.len() - 1);
        let mut fields = Vec::with_capacity(escaped.len());
        let mut malformed = None;
        for field in escaped {
            match unescape_field(field) {
                Ok(value) => fields.push(value),
                Err(e) => {
                    malformed = Some(e);
                    break;
                }
            }
        }
        if let Some(e) = malformed {
            integrity.push(format!("corrupt-record:{name}:{line} ({e})"));
            continue;
        }
        let refs: Vec<&str> = fields.iter().map(String::as_str).collect();
        if record_digest(&refs) != digest[0] {
            integrity.push(format!("corrupt-record:{name}:{line}"));
            continue;
        }
        if fields[0] != schema {
            integrity.push(format!("unknown-record-schema:{name}:{line}"));
            continue;
        }
        out.push(fields);
    }
    Ok(out)
}

/// Load one journal directory. Identity and declaration failures are hard errors.
pub fn load_journal(dir: &Path) -> Result<Journal> {
    let identity_path = dir.join("journal.json");
    let raw = read_optional(&identity_path)?.ok_or_else(|| StoreError::Rule {
        rule: "journal-identity",
        path: identity_path.clone(),
        detail: "journal.json is absent; the journal cannot be identified".to_string(),
    })?;
    let identity: IdentityFile = serde_json::from_str(&raw).map_err(|e| StoreError::Rule {
        rule: "journal-identity",
        path: identity_path.clone(),
        detail: format!("journal.json did not parse: {e}"),
    })?;
    if identity.schema != IDENTITY_SCHEMA {
        return Err(StoreError::Rule {
            rule: "journal-identity",
            path: identity_path.clone(),
            detail: format!(
                "unsupported schema {:?}; expected {IDENTITY_SCHEMA:?}",
                identity.schema
            ),
        });
    }
    let dir_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if identity.journal_id != dir_name {
        return Err(StoreError::Rule {
            rule: "journal-identity",
            path: identity_path,
            detail: format!(
                "journal_id {:?} does not match its directory {dir_name:?}",
                identity.journal_id
            ),
        });
    }

    let mut integrity = Vec::new();

    // Declarations bound the blast radius of every other kind of damage, so damage *here* cannot be
    // reported and carried — it is fatal.
    let mut keys_declared = BTreeSet::new();
    let declared_path = dir.join("keys-declared.txt");
    if let Some(raw) = read_optional(&declared_path)? {
        let (text, truncated) = complete_text(&raw);
        if truncated {
            return Err(StoreError::Rule {
                rule: "journal-declarations",
                path: declared_path.clone(),
                detail: "the declaration list ends mid-record, so the journal's declared scope is \
                         unknown and damage in it could not be bounded"
                    .to_string(),
            });
        }
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let key = CoverageKey::parse(line).map_err(|e| StoreError::Rule {
                rule: "journal-declarations",
                path: declared_path.clone(),
                detail: format!("line {}: {e}; the declared scope is unknown", i + 1),
            })?;
            keys_declared.insert(key);
        }
    }

    let mut observations = Vec::new();
    for (i, record) in read_records(
        &dir.join("observations.csv"),
        OBSERVATION_HEADER,
        OBSERVATION_ROW,
        &mut integrity,
    )?
    .into_iter()
    .enumerate()
    {
        let Ok(key) = CoverageKey::parse(&record[1]) else {
            integrity.push(format!("unparseable-key:observations.csv:{}", i + 2));
            continue;
        };
        let Some(outcome) = Outcome::parse(&record[3]) else {
            integrity.push(format!("unknown-outcome:observations.csv:{}", i + 2));
            continue;
        };
        observations.push(Observation {
            key,
            case_id: record[2].clone(),
            outcome,
            fixture_sha256: record[4].clone(),
            output_sha256: record[5].clone(),
            detail: record[6].clone(),
        });
    }

    let mut unmapped = Vec::new();
    for (i, record) in read_records(
        &dir.join("unmapped.csv"),
        UNMAPPED_HEADER,
        UNMAPPED_ROW,
        &mut integrity,
    )?
    .into_iter()
    .enumerate()
    {
        let Some(outcome) = Outcome::parse(&record[4]) else {
            integrity.push(format!("unknown-outcome:unmapped.csv:{}", i + 2));
            continue;
        };
        unmapped.push(UnmappedObservation {
            target: record[1].clone(),
            emitter_operation: record[2].clone(),
            case_id: record[3].clone(),
            outcome,
            fixture_sha256: record[5].clone(),
            unresolved: record[6].clone(),
            detail: record[7].clone(),
        });
    }

    let mut facets = Vec::new();
    for record in read_records(
        &dir.join("facets.csv"),
        FACET_HEADER,
        FACET_ROW,
        &mut integrity,
    )? {
        facets.push(Facet {
            case_id: record[1].clone(),
            key: record[2].clone(),
            name: record[3].clone(),
            value: record[4].clone(),
        });
    }

    integrity.sort();
    integrity.dedup();

    // Damage is only tolerable because the declared scope bounds it. With no declarations there is
    // nothing to withhold, so the tree fails closed rather than under-reporting.
    if !integrity.is_empty() && keys_declared.is_empty() {
        return Err(StoreError::Rule {
            rule: "journal-unbounded-damage",
            path: dir.to_path_buf(),
            detail: format!(
                "damaged records ({}) but no declarations to bound them; the affected coverage \
                 cannot be determined",
                integrity.join(", ")
            ),
        });
    }

    Ok(Journal {
        journal_id: identity.journal_id,
        emitter: identity.emitter,
        started_utc: identity.started_utc,
        host_os: identity.host_os,
        toolchain: identity.toolchain,
        scope_id: identity.scope_id,
        scope_version: identity.scope_version,
        implementation_digest: identity.implementation_digest,
        sealed_run_id: identity.sealed_run_id,
        keys_declared,
        observations,
        unmapped,
        facets,
        integrity,
    })
}

/// The journal directory of an evidence root.
pub fn journals_dir(root: &Path) -> PathBuf {
    root.join("auto").join(DIR)
}

/// Why a journal is **not** the same evidence as the run it names. Empty means it is.
///
/// A `sealed_run_id` is a claim, not a proof. Deduplicating on the strength of a matching string
/// would let a wrong, stale or planted id delete a journal's evidence outright, so the link is
/// verified: the run must agree with the journal's identity, and must contain everything the
/// journal verified.
pub fn equivalence_findings(journal: &Journal, run: &Run) -> Vec<String> {
    let mut out = Vec::new();
    let m = &run.manifest;
    for (what, a, b) in [
        ("emitter", journal.emitter.as_str(), m.emitter.as_str()),
        (
            "started_utc",
            journal.started_utc.as_str(),
            m.started_utc.as_str(),
        ),
        ("scope_id", journal.scope_id.as_str(), m.scope_id.as_str()),
        (
            "implementation_digest",
            journal.implementation_digest.as_str(),
            m.implementation_digest.as_str(),
        ),
        (
            "toolchain",
            journal.toolchain.as_str(),
            m.toolchain.as_str(),
        ),
        ("host_os", journal.host_os.as_str(), m.host_os.as_str()),
    ] {
        if a != b {
            out.push(format!("{what}({a:?} vs {b:?})"));
        }
    }
    if journal.scope_version != m.scope_version {
        out.push(format!(
            "scope_version({} vs {})",
            journal.scope_version, m.scope_version
        ));
    }

    let missing = journal
        .keys_declared
        .iter()
        .filter(|k| !run.keys_declared.contains(k))
        .count();
    if missing > 0 {
        out.push(format!("{missing} declaration(s) absent from the run"));
    }
    let missing = journal
        .observations
        .iter()
        .filter(|o| !run.observations.contains(o))
        .count();
    if missing > 0 {
        out.push(format!("{missing} observation(s) absent from the run"));
    }
    let missing = journal
        .unmapped
        .iter()
        .filter(|u| !run.unmapped.contains(u))
        .count();
    if missing > 0 {
        out.push(format!("{missing} unmapped record(s) absent from the run"));
    }
    let missing = journal
        .facets
        .iter()
        .filter(|f| !run.facets.contains(f))
        .count();
    if missing > 0 {
        out.push(format!("{missing} facet(s) absent from the run"));
    }
    out
}

/// Entry names permitted alongside journal directories in `<root>/auto/in-progress`.
///
/// Deliberately empty: nothing in this crate writes an auxiliary entry there. The emitter's only
/// temporary file, `journal.json.new`, is created *inside* a journal directory and renamed over its
/// target, so it never appears at this level. The list exists so that adding such an entry is a
/// decision recorded here rather than a silent filter in the loader.
pub const PERMITTED_AUXILIARY_ENTRIES: &[&str] = &[];

/// What a directory entry is, as far as discovery is concerned. `Other` covers symlinks (this is a
/// no-follow classification) and every non-regular entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
    Other,
}

/// Classify one entry **without following links**.
///
/// A metadata read that fails is an error, not an absence: the entry was listed, so something is
/// there, and skipping it would drop a journal — and its withholding effect — from the evidence.
pub fn read_entry_kind(path: &Path) -> Result<EntryKind> {
    let meta = std::fs::symlink_metadata(path).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    // R4: one platform policy for every evidence walker. `is_symlink` misses several Windows
    // reparse classes, so the reparse **attribute** is what decides — see
    // [`super::paths::is_reparse_point`]. A reparse point classifies as `Other` and is therefore
    // refused by `classify_entry` below, exactly as it is refused everywhere else.
    if super::paths::is_reparse_point(&meta) {
        return Ok(EntryKind::Other);
    }
    Ok(if meta.is_dir() {
        EntryKind::Dir
    } else if meta.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    })
}

/// Decide what discovery does with one entry: `Some(path)` is a journal directory to load, `None`
/// is a name explicitly permitted to be there, and an error is anything else.
///
/// Split from the filesystem so the branches that cannot be provoked reliably on every platform —
/// an unsupported link, chiefly — are still directly testable.
pub fn classify_entry(path: &Path, kind: EntryKind) -> Result<Option<PathBuf>> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match kind {
        EntryKind::Dir => Ok(Some(path.to_path_buf())),
        EntryKind::File if PERMITTED_AUXILIARY_ENTRIES.contains(&name.as_str()) => Ok(None),
        EntryKind::File => Err(StoreError::Rule {
            rule: "journal-unexpected-entry",
            path: path.to_path_buf(),
            detail: format!(
                "{name:?} is not a journal directory and is not a permitted auxiliary entry;                  ignoring it could hide evidence"
            ),
        }),
        EntryKind::Other => Err(StoreError::Rule {
            rule: "journal-unsupported-link",
            path: path.to_path_buf(),
            detail: format!(
                "{name:?} is a link or other non-regular entry; journals are never resolved                  through one, and silently skipping it could hide evidence"
            ),
        }),
    }
}

/// Load every journal under `<root>/auto/in-progress`, sorted by id, skipping only those **proved**
/// to be the same evidence as a present sealed run (see [`equivalence_findings`]).
pub fn load_journals(root: &Path, runs: &[Run]) -> Result<Vec<Journal>> {
    let dir = journals_dir(root);
    // R4: the root gets the same treatment as the entries — absent is absent, and an existing root
    // that is a file, a link or unreadable is an error rather than "no journals".
    if !super::paths::optional_dir_exists(&dir)? {
        return Ok(Vec::new());
    }
    let mut dirs = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|source| StoreError::Io {
        path: dir.clone(),
        source,
    })?;
    for entry in entries {
        // A failed *iteration* step is a failure to enumerate: the directory may hold journals this
        // loop will never see, so it cannot be treated as the end of the listing.
        let entry = entry.map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        let path = entry.path();
        if let Some(dir_path) = classify_entry(&path, read_entry_kind(&path)?)? {
            dirs.push(dir_path);
        }
    }
    dirs.sort();

    let mut out = Vec::new();
    for path in &dirs {
        let mut journal = load_journal(path)?;
        if let Some(run_id) = journal.sealed_run_id.clone() {
            // Damage is decided *before* anything is appended below, so the guard reflects the
            // journal's own condition and not the outcome of this comparison.
            let damaged = !journal.damaging_findings().is_empty();
            match runs.iter().find(|r| r.run_id() == run_id) {
                Some(run) => {
                    let findings = equivalence_findings(&journal, run);
                    if findings.is_empty() && !damaged {
                        continue; // proved identical; counting it twice would be a fabrication
                    }
                    if findings.is_empty() {
                        // Every record that survived matches the run — but the records that did
                        // *not* survive are exactly the ones equivalence could not examine, so the
                        // agreement of the remainder proves nothing about them (C0-S2-INT1-R3).
                        journal.integrity.push(format!(
                            "sealed-run-unverifiable:{run_id}:damaged records cannot be compared,                              so the surviving subset does not prove equivalence"
                        ));
                    } else {
                        journal.integrity.push(format!(
                            "sealed-run-mismatch:{run_id}:{}",
                            findings.join("; ")
                        ));
                    }
                }
                // Dropping this would discard the only surviving copy.
                None => journal
                    .integrity
                    .push(format!("sealed-run-missing:{run_id}")),
            }
        }
        journal.integrity.sort();
        journal.integrity.dedup();
        out.push(journal);
    }
    Ok(out)
}

/// SHA-256 of every file under a journal directory, for proving a read-only operation left the
/// source bytes alone.
pub fn journal_bytes(dir: &Path) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|source| StoreError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        if read_entry_kind(&entry.path())? != EntryKind::File {
            continue;
        }
        let bytes = std::fs::read(entry.path()).map_err(|source| StoreError::Io {
            path: entry.path(),
            source,
        })?;
        out.push((
            entry.file_name().to_string_lossy().into_owned(),
            store::sha256_hex(&bytes),
        ));
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_digest_is_unambiguous_where_the_separator_scheme_was_not() {
        // The exact `/2` collision: joining with 0x1f made these two field lists identical.
        assert_ne!(record_digest(&["a\u{1f}b"]), record_digest(&["a", "b"]));
        assert_ne!(record_digest(&["ab", ""]), record_digest(&["a", "b"]));
        assert_ne!(record_digest(&["", "ab"]), record_digest(&["ab"]));
        assert_eq!(record_digest(&["x", "yz"]), record_digest(&["x", "yz"]));
    }

    #[test]
    fn a_record_is_always_one_physical_line() {
        let nasty = "line1\nline2\r\n\ttab \u{1f}unit, \"quoted\"";
        let line = record_line(&[OBSERVATION_ROW, nasty]);
        assert_eq!(line.matches('\n').count(), 1, "{line:?}");
        assert!(line.ends_with('\n'));
        let parsed = csv::parse(&line).expect("parses");
        assert_eq!(unescape_field(&parsed[0][1]).expect("unescape"), nasty);
    }

    #[test]
    fn escaping_round_trips_and_rejects_malformed_input() {
        for value in ["", "plain", "a\\b", "\n\r\u{7f}\u{0}", "ünïcode ✓"] {
            assert_eq!(
                unescape_field(&escape_field(value)).expect("round trip"),
                value
            );
        }
        assert!(unescape_field("trailing\\").is_err());
        assert!(unescape_field("\\q").is_err());
        assert!(unescape_field("\\xZZ").is_err());
    }

    #[test]
    fn an_unterminated_tail_is_a_truncated_record_not_a_value() {
        assert_eq!(complete_text("a\nb\n"), ("a\nb\n".to_string(), false));
        assert_eq!(complete_text("a\nb"), ("a\n".to_string(), true));
        assert_eq!(complete_text(""), (String::new(), false));
        assert_eq!(complete_text("partial"), (String::new(), true));
    }
}
