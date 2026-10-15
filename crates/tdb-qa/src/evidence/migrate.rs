//! Conservative migration of the pre-contract evidence —
//! [contract §6 and §7.2](../../../../docs/plans/qa-evidence-contract.md).
//!
//! Migration **freezes** its sources first, maps them into non-qualifying layer-L claims, accounts
//! for every source cell, and writes only into a candidate directory. It never writes layer B, never
//! fabricates a receipt and never invents a date.
//!
//! The rules this module implements, and where each is enforced:
//!
//! | Rule | Here |
//! |---|---|
//! | **R-FREEZE** — sources and the ordering used to read them are frozen and hashed; a changed catalog refuses | [`freeze`], [`verify_freeze`] |
//! | **R-POSITIONAL** — the family is a positional assumption, labelled as such; a content-contradicting row goes to `_unmapped/` | [`migrate`] |
//! | **R-CLAIM** — legacy labels become claims with no receipts, no run ids and no synthesized dates | [`migrate`] |
//! | **R-ACCOUNT** — every source cell gets exactly one accounting row | [`migrate`] |
//! | **R-NO-BACKFILL** — the destination-level `heads` row is never expanded onto pair keys | [`migrate`], [`headmesh_pair_keys`] |
//! | **R-UNKNOWN / R-CONV** — a conversion component the registry does not record is the literal `unknown`, and two pairs rendering one key are refused, never merged | [`headmesh_pair_keys`] |
//! | **R-DUP** — both duplicate asset matrices are preserved; their differences are published | [`divergence`] |
//! | **R-MIG-SEP** — `install` is the only path into layer B and refuses without a matching approval digest | [`install`] |

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::csv;
use super::key::CoverageKey;
use super::paths::{self, Layout};
use super::publish::DivergenceRow;
use super::store::{self, sha256_hex, sha256_prefixed, StoreError};

type Result<T> = std::result::Result<T, StoreError>;

fn io(path: &Path) -> impl Fn(std::io::Error) -> StoreError + '_ {
    move |source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn rule(rule: &'static str, path: &Path, detail: impl Into<String>) -> StoreError {
    StoreError::Rule {
        rule,
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

/// The three legacy sources, as `(frozen name, repo-relative path)`.
pub const SOURCES: &[(&str, &str)] = &[
    (
        "game-qa-matrix.frozen.csv",
        "docs/audits/data/game-qa-matrix.csv",
    ),
    (
        "capability.frozen.csv",
        "docs/audits/data/assets/nhllegacy/xbox360/capability.csv",
    ),
    (
        "asset-capability-matrix.frozen.csv",
        "docs/audits/data/asset-capability-matrix.csv",
    ),
];

/// `legacy/frozen/freeze.json` — the sources **and the ordering used to interpret them**.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Freeze {
    pub schema: String,
    /// Recorded verbatim; never read by a decision (R-NO-TIME).
    pub frozen_utc: String,
    pub sources: BTreeMap<String, String>,
    pub family_order: Vec<String>,
    pub family_order_sha256: String,
    pub unit_order: Vec<String>,
    pub unit_order_sha256: String,
}

fn order_digest(items: &[String]) -> String {
    sha256_hex(items.join("\n").as_bytes())
}

/// The live catalog's family and unit ordering, which is what a frozen file's row positions mean.
fn live_orders() -> (Vec<String>, Vec<String>) {
    (
        crate::catalog::FAMILIES
            .iter()
            .map(|f| f.id.to_string())
            .collect(),
        crate::catalog::CATALOG
            .iter()
            .map(|u| format!("{}-{}", u.game, u.platform))
            .collect(),
    )
}

/// Copy each legacy source byte-for-byte into `frozen_dir` and record its digest together with the
/// catalog ordering used to interpret it (**R-FREEZE**).
pub fn freeze(repo_root: &Path, frozen_dir: &Path, frozen_utc: &str) -> Result<Freeze> {
    fs::create_dir_all(frozen_dir).map_err(io(frozen_dir))?;
    let mut sources = BTreeMap::new();
    for (name, rel) in SOURCES {
        let from = repo_root.join(rel);
        let bytes = fs::read(&from).map_err(io(&from))?;
        paths::write_file(&frozen_dir.join(name), &bytes)?;
        sources.insert((*name).to_string(), sha256_hex(&bytes));
    }
    let (family_order, unit_order) = live_orders();
    let freeze = Freeze {
        schema: "qa-legacy-freeze/1".into(),
        frozen_utc: frozen_utc.to_string(),
        family_order_sha256: order_digest(&family_order),
        unit_order_sha256: order_digest(&unit_order),
        family_order,
        sources,
        unit_order,
    };
    let path = frozen_dir.join("freeze.json");
    paths::write_file(
        &path,
        serde_json::to_string_pretty(&freeze)
            .map_err(|e| rule("freeze", &path, e.to_string()))?
            .as_bytes(),
    )?;
    Ok(freeze)
}

/// Load a freeze and refuse if the live catalog no longer matches the frozen ordering
/// (**R-FREEZE**, acceptance case A25), or if a frozen source's bytes have changed.
pub fn verify_freeze(frozen_dir: &Path) -> Result<Freeze> {
    let path = frozen_dir.join("freeze.json");
    let freeze: Freeze = serde_json::from_str(&fs::read_to_string(&path).map_err(io(&path))?)
        .map_err(|e| rule("freeze", &path, format!("did not parse: {e}")))?;
    let (family_order, unit_order) = live_orders();
    if order_digest(&family_order) != freeze.family_order_sha256 {
        return Err(rule(
            "R-FREEZE",
            &path,
            "the live catalog's family order no longer matches the frozen order; \
             migrating would silently re-interpret every frozen row",
        ));
    }
    if order_digest(&unit_order) != freeze.unit_order_sha256 {
        return Err(rule(
            "R-FREEZE",
            &path,
            "the live catalog's unit order no longer matches the frozen order",
        ));
    }
    for (name, digest) in &freeze.sources {
        let file = frozen_dir.join(name);
        let actual = sha256_hex(&fs::read(&file).map_err(io(&file))?);
        if &actual != digest {
            return Err(rule(
                "R-FREEZE",
                &file,
                format!("frozen source changed: {digest} -> {actual}"),
            ));
        }
    }
    Ok(freeze)
}

/// What one migration produced. Counts are asserted against the sources, not assumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub migration_id: String,
    pub claims: usize,
    pub unmapped: usize,
    /// Cells counted in the frozen sources.
    pub source_cells: usize,
    /// Rows written to `migration-accounting.csv`; must equal `source_cells` (A12).
    pub accounting_rows: usize,
    pub divergence_rows: usize,
    /// The digest an approval file must name to install this candidate.
    pub candidate_set_sha256: String,
    /// Migration produces **no** manual records (R-CLAIM). Always 0 here; the field exists so the
    /// number is stated rather than assumed.
    pub records: usize,
}

const ACCOUNTING_HEADER: &[&str] = &[
    "schema",
    "source_file",
    "source_row",
    "source_column",
    "source_value_raw",
    "disposition",
    "claim_id",
    "unresolved",
];

/// The only family mappings taken from a legacy asset-family token. A token with no **exact**
/// counterpart is left unmapped with a recorded reason: guessing which coverage family a legacy
/// label meant is exactly the invention this contract forbids.
fn map_asset_family(token: &str) -> Option<&'static str> {
    let candidate = format!("assets-{token}");
    crate::catalog::FAMILIES
        .iter()
        .find(|f| f.id == candidate)
        .map(|f| f.id)
}

fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "")
}

#[allow(clippy::too_many_arguments)]
fn claim_toml(
    claim_id: &str,
    source_file: &str,
    source_sha256: &str,
    source_row: usize,
    source_row_raw: &str,
    status_raw: &str,
    last_verified_raw: &str,
    evidence_raw: &str,
    maintenance: &str,
    target: Option<&str>,
    family: Option<&str>,
    family_source: &str,
    confidence: &str,
    unresolved: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("schema = \"qa-legacy-claim/1\"\n");
    out.push_str(&format!("claim_id = \"{}\"\n", toml_escape(claim_id)));
    out.push_str(&format!("source_file = \"{}\"\n", toml_escape(source_file)));
    out.push_str(&format!("source_sha256 = \"{source_sha256}\"\n"));
    out.push_str(&format!("source_row = {source_row}\n"));
    out.push_str(&format!(
        "source_row_raw = \"{}\"\n",
        toml_escape(source_row_raw)
    ));
    out.push_str(&format!(
        "claim_status_raw = \"{}\"\n",
        toml_escape(status_raw)
    ));
    out.push_str(&format!(
        "claim_last_verified_raw = \"{}\"\n",
        toml_escape(last_verified_raw)
    ));
    out.push_str(&format!(
        "claim_evidence_raw = \"{}\"\n",
        toml_escape(evidence_raw)
    ));
    out.push_str(&format!("maintenance = \"{maintenance}\"\n"));
    // R-CLAIM: structurally false, and there is no field by which it could become true.
    out.push_str("qualifies = false\n\n[mapping]\n");
    match target {
        Some(t) => out.push_str(&format!("target = \"{t}\"\n")),
        None => out.push_str("# target: unresolved — not written rather than guessed\n"),
    }
    match family {
        Some(f) => out.push_str(&format!("family = \"{f}\"\n")),
        None => out.push_str("# family: unresolved — not written rather than guessed\n"),
    }
    out.push_str(&format!("family_source = \"{family_source}\"\n"));
    out.push_str(&format!("confidence = \"{confidence}\"\n"));
    out.push_str("operation = \"unknown\"\n");
    let tokens: Vec<String> = unresolved
        .iter()
        .map(|u| format!("\"{}\"", toml_escape(u)))
        .collect();
    out.push_str(&format!("unresolved = [{}]\n", tokens.join(", ")));
    out
}

/// Run the migration into an isolated candidate directory. Nothing outside `candidate_dir` is
/// written, and `manual/` is never touched.
/// **R2:** a validated [`Layout`] is required. The R1 `migrate` wrapper passed no protection at
/// all, so a library caller could render a candidate straight into the evidence tree; that wrapper
/// is gone and this is the only entry point.
pub fn migrate(
    frozen_dir: &Path,
    candidate_dir: &Path,
    migration_id: &str,
    layout: &Layout,
) -> Result<MigrationReport> {
    // Checked before the candidate directory is created (R1).
    let resolved_candidate =
        layout.assert_output_is_safe("the migration candidate", candidate_dir)?;
    let resolved_frozen = paths::resolve_for_write(frozen_dir)?;
    paths::assert_disjoint(
        "the migration candidate",
        &resolved_candidate,
        "the frozen legacy sources",
        &resolved_frozen,
    )?;

    let freeze = verify_freeze(frozen_dir)?;
    fs::create_dir_all(candidate_dir.join("claims")).map_err(io(candidate_dir))?;
    fs::create_dir_all(candidate_dir.join("_unmapped")).map_err(io(candidate_dir))?;
    fs::create_dir_all(candidate_dir.join("records")).map_err(io(candidate_dir))?;

    let mut accounting = csv::row(ACCOUNTING_HEADER);
    let mut accounting_rows = 0usize;
    let mut source_cells = 0usize;
    let mut claims = 0usize;
    let mut unmapped = 0usize;

    // ---------------------------------------------------------- game-qa-matrix
    let name = "game-qa-matrix.frozen.csv";
    let path = frozen_dir.join(name);
    let text = fs::read_to_string(&path).map_err(io(&path))?;
    let sha = freeze.sources.get(name).cloned().unwrap_or_default();
    let records = csv::parse(&text).map_err(|e| rule("migrate", &path, e.to_string()))?;
    let header = records.first().cloned().unwrap_or_default();
    for (index, record) in records.iter().skip(1).enumerate() {
        source_cells += record.len();
        let unit_index = index / freeze.family_order.len();
        let family_index = index % freeze.family_order.len();
        let unit = freeze
            .unit_order
            .get(unit_index)
            .cloned()
            .unwrap_or_default();
        let family = freeze
            .family_order
            .get(family_index)
            .cloned()
            .unwrap_or_default();

        // R-POSITIONAL: the row's own content is checked against its positional unit. A row whose
        // game/platform columns disagree is emitted without a family and preserved in `_unmapped/`.
        let row_unit = format!(
            "{}-{}",
            record.first().cloned().unwrap_or_default(),
            record.get(1).cloned().unwrap_or_default()
        );
        let contradicts = row_unit != unit;
        let mut unresolved = vec!["positional-family-assumption".to_string()];
        if contradicts {
            unresolved.push("positional-conflict".to_string());
        }
        let status = record.get(4).cloned().unwrap_or_default();
        let evidence = record.get(8).cloned().unwrap_or_default();
        let blocker = record.get(9).cloned().unwrap_or_default();
        if !blocker.is_empty() {
            unresolved.push("blocker-text-preserved".to_string());
        }
        let claim_id = format!("gameqa-row{:04}", index + 1);
        let toml = claim_toml(
            &claim_id,
            name,
            &sha,
            index + 2,
            &record.join(","),
            &status,
            &record.get(7).cloned().unwrap_or_default(),
            &evidence,
            "generated",
            (!contradicts).then_some(row_unit.as_str()),
            (!contradicts).then_some(family.as_str()),
            "positional-assumption",
            "positional",
            &unresolved,
        );
        let dir = if contradicts { "_unmapped" } else { "claims" };
        let out = candidate_dir.join(dir).join(format!("{claim_id}.toml"));
        paths::write_file(&out, toml.as_bytes())?;
        if contradicts {
            unmapped += 1;
        } else {
            claims += 1;
        }

        // R-ACCOUNT — one row per cell, including every blank.
        for (column_index, value) in record.iter().enumerate() {
            let column = header.get(column_index).cloned().unwrap_or_default();
            let disposition = if value.is_empty() {
                "preserved-no-evidence"
            } else if column == "status" || column == "game" || column == "platform" {
                "mapped"
            } else {
                "preserved-raw"
            };
            let token = if contradicts {
                "positional-conflict"
            } else {
                "none"
            };
            accounting.push_str(&csv::row([
                "qa-migration-accounting/1",
                name,
                &(index + 2).to_string(),
                &column,
                value,
                disposition,
                &claim_id,
                token,
            ]));
            accounting_rows += 1;
        }
    }

    // -------------------------------------------------- the two asset matrices
    for (name, id_prefix, family_column, target) in [
        (
            "capability.frozen.csv",
            "cap",
            "asset_family",
            "nhllegacy-xbox360",
        ),
        (
            "asset-capability-matrix.frozen.csv",
            "assetmatrix",
            "asset_family",
            "",
        ),
    ] {
        let path = frozen_dir.join(name);
        let text = fs::read_to_string(&path).map_err(io(&path))?;
        let sha = freeze.sources.get(name).cloned().unwrap_or_default();
        let records = csv::parse(&text).map_err(|e| rule("migrate", &path, e.to_string()))?;
        let header = records.first().cloned().unwrap_or_default();
        let family_at = header.iter().position(|c| c == family_column);
        for (index, record) in records.iter().skip(1).enumerate() {
            source_cells += record.len();
            let token = family_at
                .and_then(|i| record.get(i))
                .cloned()
                .unwrap_or_default();
            let mapped = map_asset_family(&token);
            let mut unresolved = Vec::new();
            if mapped.is_none() {
                unresolved.push(format!("family-not-mapped:{token}"));
            }
            if token == "heads" {
                // E11 / R-NO-BACKFILL — destination granularity only.
                unresolved.push("granularity-destination".to_string());
                unresolved.push("not-pair-specific".to_string());
            }
            // The row's own target, when the file carries one.
            let row_target = if target.is_empty() {
                let game = header
                    .iter()
                    .position(|c| c == "game_id")
                    .and_then(|i| record.get(i));
                let platform = header
                    .iter()
                    .position(|c| c == "platform")
                    .and_then(|i| record.get(i));
                match (game, platform) {
                    (Some(g), Some(p)) => Some(format!("{g}-{p}")),
                    _ => None,
                }
            } else {
                Some(target.to_string())
            };
            let claim_id = format!(
                "{id_prefix}-{}-{}",
                row_target.clone().unwrap_or_default(),
                token
            );
            let toml = claim_toml(
                &claim_id,
                name,
                &sha,
                index + 2,
                &record.join(","),
                &token,
                "unknown",
                "",
                "hand",
                row_target.as_deref(),
                mapped,
                "declared-column",
                "row-level",
                &unresolved,
            );
            let dir = if mapped.is_some() {
                "claims"
            } else {
                "_unmapped"
            };
            let out = candidate_dir.join(dir).join(format!("{claim_id}.toml"));
            paths::write_file(&out, toml.as_bytes())?;
            if mapped.is_some() {
                claims += 1;
            } else {
                unmapped += 1;
            }

            for (column_index, value) in record.iter().enumerate() {
                let column = header.get(column_index).cloned().unwrap_or_default();
                // §6.4 — every one of these is preserved verbatim, never rounded or reinterpreted.
                let (disposition, note) = match (column.as_str(), value.as_str()) {
                    (_, "") => ("preserved-no-evidence", "none".to_string()),
                    (_, "not_run") => ("preserved-no-evidence", "none".to_string()),
                    ("install", "external") => {
                        ("preserved-raw", "external-meaning-undefined".to_string())
                    }
                    ("format", _) => ("preserved-raw", "variant-not-mapped".to_string()),
                    (_, "partial") => ("preserved-raw", "partial-not-rounded".to_string()),
                    ("asset_family", _) if mapped.is_none() => {
                        ("unmapped", format!("family-not-mapped:{value}"))
                    }
                    ("asset_family", _) => ("mapped", "none".to_string()),
                    _ => ("preserved-raw", "none".to_string()),
                };
                accounting.push_str(&csv::row([
                    "qa-migration-accounting/1",
                    name,
                    &(index + 2).to_string(),
                    &column,
                    value,
                    disposition,
                    &claim_id,
                    &note,
                ]));
                accounting_rows += 1;
            }
        }
    }

    let accounting_path = candidate_dir.join("migration-accounting.csv");
    paths::write_file(&accounting_path, accounting.as_bytes())?;

    // R-DUP — publish the differences; neither matrix wins.
    let divergence_rows = divergence(frozen_dir)?;
    let mut divergence_csv = csv::row([
        "schema",
        "subject",
        "left_source",
        "left_value",
        "right_source",
        "right_value",
        "unresolved",
    ]);
    for d in &divergence_rows {
        divergence_csv.push_str(&csv::row([
            "qa-asset-matrix-divergence/1",
            &d.subject,
            &d.left_source,
            &d.left_value,
            &d.right_source,
            &d.right_value,
            &d.unresolved,
        ]));
    }
    let divergence_path = candidate_dir.join("divergence.csv");
    paths::write_file(&divergence_path, divergence_csv.as_bytes())?;

    let candidate_set_sha256 = candidate_digest(candidate_dir)?;
    Ok(MigrationReport {
        migration_id: migration_id.to_string(),
        claims,
        unmapped,
        source_cells,
        accounting_rows,
        divergence_rows: divergence_rows.len(),
        candidate_set_sha256,
        records: 0,
    })
}

/// A digest over every file in a candidate directory, path and content.
pub fn candidate_digest(candidate_dir: &Path) -> Result<String> {
    let mut joined = String::new();
    for path in walk(candidate_dir)? {
        let rel = path
            .strip_prefix(candidate_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if rel == "approval.toml" || rel == "candidate.json" {
            continue;
        }
        joined.push_str(&rel);
        joined.push('\0');
        joined.push_str(&sha256_hex(&fs::read(&path).map_err(io(&path))?));
        joined.push('\0');
    }
    Ok(sha256_prefixed(joined.as_bytes()))
}

/// The migration digest's input tree. R4: the shared fail-closed traversal, so a link, an
/// unreadable entry or a failed enumeration cannot quietly shrink the set of files a digest is
/// taken over — which would have produced a *different but plausible* digest rather than an error.
fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
    super::paths::walk_files(dir)
}

/// **R-DUP**: the differences between the two hand-maintained asset matrices. Neither wins by
/// default; a subject present in only one is reported as such, not resolved.
pub fn divergence(frozen_dir: &Path) -> Result<Vec<DivergenceRow>> {
    let left_name = "capability.frozen.csv";
    let right_name = "asset-capability-matrix.frozen.csv";
    let left = read_keyed(&frozen_dir.join(left_name), "asset_family")?;
    let right = read_keyed(&frozen_dir.join(right_name), "asset_family")?;
    let subjects: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
    let mut out = Vec::new();
    for subject in subjects {
        let l = left.get(subject);
        let r = right.get(subject);
        match (l, r) {
            (Some(l), Some(r)) => {
                let columns: BTreeSet<&String> = l.keys().chain(r.keys()).collect();
                for column in columns {
                    let lv = l.get(column).cloned().unwrap_or_default();
                    let rv = r.get(column).cloned().unwrap_or_default();
                    if lv != rv {
                        out.push(DivergenceRow {
                            subject: format!("{subject}/{column}"),
                            left_source: left_name.into(),
                            left_value: lv,
                            right_source: right_name.into(),
                            right_value: rv,
                            unresolved: "duplicate-source".into(),
                        });
                    }
                }
            }
            (Some(_), None) => out.push(DivergenceRow {
                subject: subject.clone(),
                left_source: left_name.into(),
                left_value: "present".into(),
                right_source: right_name.into(),
                right_value: "absent".into(),
                unresolved: "duplicate-source-row-missing".into(),
            }),
            (None, Some(_)) => out.push(DivergenceRow {
                subject: subject.clone(),
                left_source: left_name.into(),
                left_value: "absent".into(),
                right_source: right_name.into(),
                right_value: "present".into(),
                unresolved: "duplicate-source-row-missing".into(),
            }),
            (None, None) => {}
        }
    }
    out.sort_by(|a, b| a.subject.cmp(&b.subject));
    Ok(out)
}

fn read_keyed(path: &Path, key_column: &str) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    let text = fs::read_to_string(path).map_err(io(path))?;
    let records = csv::parse(&text).map_err(|e| rule("divergence", path, e.to_string()))?;
    let header = records.first().cloned().unwrap_or_default();
    let key_at = header.iter().position(|c| c == key_column);
    let mut out = BTreeMap::new();
    for record in records.iter().skip(1) {
        let Some(key) = key_at.and_then(|i| record.get(i)).cloned() else {
            continue;
        };
        let mut fields = BTreeMap::new();
        for (i, value) in record.iter().enumerate() {
            if let Some(column) = header.get(i) {
                fields.insert(column.clone(), value.clone());
            }
        }
        out.insert(key, fields);
    }
    Ok(out)
}

// ------------------------------------------------------------- headmesh ------

/// The conversion pair keys declared by `defs/appearance/headmesh-matrix.toml`, for one operation.
///
/// **R-NO-BACKFILL**: these keys are derived from the recipe registry only. The destination-level
/// `heads` claim migrated from `capability.csv` is never copied onto any of them, so a pair starts
/// `unqualified` — which is correct, because coarse evidence never was pair-specific proof.
///
/// **R-UNKNOWN (C0-MIG-ID1).** The registry records a *source*, a *destination* and a *conversion
/// path*. It records neither a conversion **class** nor a **recipe id**, so both are emitted as the
/// contract's literal `unknown` — which keeps the key valid and **blocks qualification** (§2.2
/// R-CONV, merge rows 7 and 8). Earlier revisions wrote `class = "head-authored"` for every pair
/// and re-used `id_scheme` as the recipe; both are known-*looking* tokens for values nothing
/// records, and, unlike `unknown`, neither blocks qualification. The class vocabulary is still open
/// question **U2**: until it closes there is no vocabulary a class could honestly be drawn from.
///
/// **R-CONV collision rejection.** Two `[[pair]]` entries that render one key are a failure of the
/// whole operation, naming both declaration positions and the key. Nothing is deduplicated,
/// dropped, or handed an invented ordinal recipe id: two pairs the registry cannot distinguish are
/// *unresolved identity*, and this error is how that is reported.
pub fn headmesh_pair_keys(matrix_path: &Path, operation: &str) -> Result<Vec<CoverageKey>> {
    let text = fs::read_to_string(matrix_path).map_err(io(matrix_path))?;
    let value: toml::Value = text
        .parse()
        .map_err(|e| rule("headmesh", matrix_path, format!("TOML: {e}")))?;
    let pairs = value
        .get("pair")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();
    // Declaration order is kept until the collision check has run, so a diagnostic can name the
    // `[[pair]]` positions a reader has to go and look at.
    let mut out: Vec<(usize, CoverageKey)> = Vec::new();
    for (index, pair) in pairs.iter().enumerate() {
        let position = index + 1;
        let field = |name: &str| {
            pair.get(name)
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string()
        };
        // An absent or malformed `target` stays fail-closed: `unknown` is not `<title>-<platform>`,
        // so the key does not parse and the whole registry is refused rather than half-mapped.
        let target = field("target");
        let raw = format!(
            "{target}/assets-heads/{operation}?from={};class=unknown;path={};recipe=unknown",
            field("source"),
            field("path"),
        );
        let key = CoverageKey::parse(&raw)
            .map_err(|e| rule("headmesh", matrix_path, format!("[[pair]] {position}: {e}")))?;
        out.push((position, key));
    }

    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (position, key) in &out {
        let rendered = key.to_string();
        if let Some(first) = seen.insert(rendered.clone(), *position) {
            let unresolved = key.conversion.as_ref().is_some_and(|c| !c.is_fully_known());
            return Err(rule(
                "R-CONV",
                matrix_path,
                format!(
                    "[[pair]] {first} and [[pair]] {position} render one coverage key {rendered}\
                     {}; the registry does not distinguish them, so neither pair is deduplicated, \
                     dropped, nor given an invented recipe id",
                    if unresolved {
                        ", whose conversion identity is unresolved (an `unknown` component)"
                    } else {
                        ""
                    }
                ),
            ));
        }
    }

    let mut keys: Vec<CoverageKey> = out.into_iter().map(|(_, key)| key).collect();
    keys.sort();
    Ok(keys)
}

// -------------------------------------------------------------- install ------

/// `approval.toml` — the file `install` refuses without.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Approval {
    pub schema: String,
    pub migration_id: String,
    pub candidate_set_sha256: String,
    pub approved_by: String,
    pub approved_utc: String,
    pub accepted_records: usize,
    pub accepted_claims: usize,
}

/// What an install did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    pub claims_installed: usize,
    pub records_installed: usize,
}

/// **R-MIG-SEP** — the sole exception to R-SEP, and the only path into layer B.
///
/// Refuses unless the approval names this migration id **and** carries the exact candidate-set
/// digest. It is never invoked by a test of another crate, by a build or by CI, and it never
/// modifies or deletes the frozen sources.
pub fn install(
    candidate_dir: &Path,
    approval_path: &Path,
    root: &Path,
    migration_id: &str,
) -> Result<InstallReport> {
    // R1: the candidate must not live inside the evidence root it is about to write into, or the
    // install would read the files it is creating.
    //
    // **R2:** the check is made against the whole validated layout rather than the root alone, so a
    // candidate sitting inside `manual/`, `legacy/frozen/` or `auto/runs/` is refused by name and
    // the diagnostic says which protected location it collided with. `install` legitimately writes
    // *into* the evidence root — it is the sole R-MIG-SEP exception — so the layout bounds its
    // **source**, not its destination.
    let layout = Layout::new(root)?;
    layout.assert_output_is_safe("the migration candidate", candidate_dir)?;

    let approval: Approval = toml::from_str(
        &fs::read_to_string(approval_path).map_err(io(approval_path))?,
    )
    .map_err(|e| {
        rule(
            "R-MIG-SEP",
            approval_path,
            format!("approval did not parse: {e}"),
        )
    })?;
    if approval.schema != "qa-migration-approval/1" {
        return Err(rule(
            "R-MIG-SEP",
            approval_path,
            format!("unsupported schema {:?}", approval.schema),
        ));
    }
    if approval.migration_id != migration_id {
        return Err(rule(
            "R-MIG-SEP",
            approval_path,
            format!(
                "approval names migration {:?}, not {migration_id:?}",
                approval.migration_id
            ),
        ));
    }
    let actual = candidate_digest(candidate_dir)?;
    if approval.candidate_set_sha256 != actual {
        return Err(rule(
            "R-MIG-SEP",
            approval_path,
            format!(
                "candidate digest mismatch; approval names {} and the candidate is {actual}",
                approval.candidate_set_sha256
            ),
        ));
    }

    let mut claims_installed = 0usize;
    for path in walk(&candidate_dir.join("claims"))? {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let to = root.join("legacy").join("claims").join(&name);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(io(parent))?;
        }
        if to.exists() {
            return Err(rule("R-MIG-SEP", &to, "a claim file is never overwritten"));
        }
        paths::copy_file(&path, &to)?;
        claims_installed += 1;
    }

    // Migration produces no manual records (R-CLAIM), so this loop normally installs nothing. The
    // path exists because an approved candidate *may* carry hand-authored records, and this is the
    // only place in the crate that writes under `manual/`.
    let mut records_installed = 0usize;
    for path in walk(&candidate_dir.join("records"))? {
        let text = fs::read_to_string(&path).map_err(io(&path))?;
        let record = store::parse_manual(&path, &text)?;
        store::write_manual_record(root, &record, &text)?;
        records_installed += 1;
    }

    Ok(InstallReport {
        claims_installed,
        records_installed,
    })
}
