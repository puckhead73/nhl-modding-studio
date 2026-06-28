//! The view-model/internal ↔ DTO bridge. Keeps all domain→wire mapping in one place, including the
//! `raw_id` string helpers carried over from the prior app.

use studio_domain::{
    DbOverview, EditSession, Finding, IntegrityReport, OpOutcome, Severity, Source,
};

use crate::dto::{
    CreatePlayerDto, FindingDto, IntegrityReportDto, OpOutcomeDto, ProjectSummaryDto,
    TableSummaryDto,
};

/// Build the project summary DTO for a session (reflects the current working db, incl. any growth).
pub fn project_summary_dto(
    project_id: u32,
    session: &EditSession,
) -> Result<ProjectSummaryDto, studio_domain::DomainError> {
    let overview = session.overview()?;
    let project = session.project();
    Ok(ProjectSummaryDto {
        project_id,
        platform: project.platform.as_str().to_string(),
        role: project.role.as_str().to_string(),
        source: source_str(project.source()).to_string(),
        endian: format!("{:?}", overview.endian),
        table_count: overview.table_count,
        db_size: overview.db_size,
        header_crc_ok: overview.header_crc_ok,
        size_ok: overview.size_ok,
        dirty: session.is_dirty(),
        tables: table_dtos(&overview),
    })
}

fn table_dtos(overview: &DbOverview) -> Vec<TableSummaryDto> {
    overview
        .tables
        .iter()
        .map(|t| TableSummaryDto {
            id: t.id.clone(),
            field_count: t.field_count,
            records: t.records,
        })
        .collect()
}

/// Map a Layer 4 report to its DTO (with per-severity tallies precomputed for the UI).
pub fn report_dto(report: &IntegrityReport) -> IntegrityReportDto {
    IntegrityReportDto {
        clean: report.is_clean(),
        error_count: report.count(Severity::Error),
        warning_count: report.count(Severity::Warning),
        info_count: report.count(Severity::Info),
        findings: report.findings.iter().map(finding_dto).collect(),
    }
}

fn finding_dto(f: &Finding) -> FindingDto {
    FindingDto {
        severity: severity_str(f.severity).to_string(),
        predicate: f.predicate.to_string(),
        message: f.message.clone(),
        table: f.table.clone(),
        field: f.field.clone(),
        record: f.record,
    }
}

pub fn outcome_dto(outcome: OpOutcome) -> OpOutcomeDto {
    OpOutcomeDto {
        edits: outcome.edits,
        repairs: outcome.repairs,
    }
}

pub fn create_player_dto(player_id: u64, outcome: OpOutcome) -> CreatePlayerDto {
    CreatePlayerDto {
        player_id,
        outcome: outcome_dto(outcome),
    }
}

fn severity_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn source_str(source: &Source) -> &'static str {
    match source {
        Source::RawDb => "raw_db",
        Source::Container { .. } => "container",
    }
}

/// Render a 4-byte on-disk id as its character string (the prior app's `raw_id_to_string`).
pub fn raw_id_to_string(id: [u8; 4]) -> String {
    id.iter().map(|&b| b as char).collect()
}

/// Parse a 4-char id string back to its on-disk bytes (the prior app's `parse_raw_id`).
pub fn parse_raw_id(s: &str) -> Option<[u8; 4]> {
    let b = s.as_bytes();
    if b.len() == 4 {
        Some([b[0], b[1], b[2], b[3]])
    } else {
        None
    }
}
