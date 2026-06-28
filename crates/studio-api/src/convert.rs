//! The view-model/internal ↔ DTO bridge. Keeps all domain→wire mapping in one place, including the
//! `raw_id` string helpers carried over from the prior app.

use studio_domain::{
    ArenaRef, ArenaSummary, ArenaView, DbOverview, EditSession, FieldRaw, FieldView, Finding,
    IntegrityReport, OpOutcome, PlayerSummary, PlayerView, RosterMembership, Severity, Source,
    TeamSummary, TeamView,
};

use crate::dto::{
    ArenaRefDto, ArenaSummaryDto, ArenaViewDto, CreatePlayerDto, FieldViewDto, FindingDto,
    IntegrityReportDto, OpOutcomeDto, PlayerSummaryDto, PlayerViewDto, ProjectSummaryDto,
    RosterMembershipDto, TableSummaryDto, TeamSummaryDto, TeamViewDto,
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

/// Map a view-model field to its DTO (splitting the raw value into `raw_int`/`text`).
pub fn field_view_dto(fv: &FieldView) -> FieldViewDto {
    let (raw_int, text) = match &fv.raw {
        FieldRaw::Int(v) => (Some(*v), None),
        FieldRaw::Text(s) => (None, Some(s.clone())),
    };
    FieldViewDto {
        studio_table: fv.studio_table.clone(),
        studio_field: fv.studio_field.clone(),
        name: fv.name.clone(),
        kind: fv.kind.as_str().to_string(),
        display: fv.display.clone(),
        decoded: fv.decoded,
        enum_label: fv.enum_label.clone(),
        raw_int,
        text,
        range_low: fv.range_low,
        range_high: fv.range_high,
        editable: fv.editable,
        is_string: fv.is_string,
    }
}

pub fn player_summary_dto(s: &PlayerSummary) -> PlayerSummaryDto {
    PlayerSummaryDto {
        player_id: s.player_id,
        first_name: s.first_name.clone(),
        last_name: s.last_name.clone(),
        position_label: s.position_label.clone(),
        team_id: s.team_id,
    }
}

fn membership_dto(m: &RosterMembership) -> RosterMembershipDto {
    RosterMembershipDto {
        team_id: m.team_id,
        jersey: m.jersey,
        captaincy: m.captaincy,
        roster_status: m.roster_status,
    }
}

pub fn player_view_dto(v: &PlayerView) -> PlayerViewDto {
    PlayerViewDto {
        player_id: v.player_id,
        bio_record: v.bio_record,
        kind: v.kind.to_string(),
        identity: v.identity.iter().map(field_view_dto).collect(),
        ratings: v.ratings.iter().map(field_view_dto).collect(),
        ratings_record: v.ratings_record,
        ratings_table: v.ratings_table.to_string(),
        memberships: v.memberships.iter().map(membership_dto).collect(),
        all_fields: v.all_fields.iter().map(field_view_dto).collect(),
    }
}

pub fn team_summary_dto(t: &TeamSummary) -> TeamSummaryDto {
    TeamSummaryDto {
        team_id: t.team_id,
        record: t.record,
        name: t.name.clone(),
        abbrev: t.abbrev.clone(),
    }
}

fn arena_ref_dto(a: &ArenaRef) -> ArenaRefDto {
    ArenaRefDto {
        arena_id: a.arena_id,
        name: a.name.clone(),
        city: a.city.clone(),
    }
}

pub fn team_view_dto(v: &TeamView) -> TeamViewDto {
    TeamViewDto {
        team_id: v.team_id,
        record: v.record,
        identity: v.identity.iter().map(field_view_dto).collect(),
        arena: v.arena.as_ref().map(arena_ref_dto),
        roster_count: v.roster_count,
        all_fields: v.all_fields.iter().map(field_view_dto).collect(),
    }
}

pub fn arena_summary_dto(a: &ArenaSummary) -> ArenaSummaryDto {
    ArenaSummaryDto {
        arena_id: a.arena_id,
        record: a.record,
        name: a.name.clone(),
        city: a.city.clone(),
    }
}

pub fn arena_view_dto(v: &ArenaView) -> ArenaViewDto {
    ArenaViewDto {
        arena_id: v.arena_id,
        record: v.record,
        identity: v.identity.iter().map(field_view_dto).collect(),
        all_fields: v.all_fields.iter().map(field_view_dto).collect(),
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
