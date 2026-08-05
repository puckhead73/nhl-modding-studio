//! The view-model/internal ↔ DTO bridge. Keeps all domain→wire mapping in one place, including the
//! `raw_id` string helpers carried over from the prior app.

use studio_domain::{
    ArenaRef, ArenaSummary, ArenaView, AssetExportSummary, AssetRoots, AttributeView,
    BrowseListing, Capabilities, CareerStats, CellControl, DbOverview, EntityFieldProfile,
    ExportOptions, ExtractSummary, FieldRaw, FieldView, FileExportOptions, FileExportSummary,
    HeadPreview, HumanFinding, HumanReport, ImportIdentity, ImportItem, InstallReport, ItemOutcome,
    LineEntry, LinesView, LoweredItem, MatchCandidate, MatchConfidence, OpOutcome,
    OutfitExportSummary, PlayerAssets, PlayerAttributes, PlayerData, PlayerHeader, PlayerSummary,
    PlayerView, ProfileCell, RosterMembership, SaveSlot, Severity, Source, SurfaceCapabilities,
    SurfaceCapability, SurfaceState, TableCapacity, TeamSummary, TeamView, TraitOption, TraitView,
    Workspace,
};

use crate::dto::{
    ArenaRefDto, ArenaSummaryDto, ArenaViewDto, AssetExportSummaryDto, AssetRootsDto,
    AttributeViewDto, BrowseEntryDto, BrowseListingDto, CapabilitiesDto, CareerColumnDto,
    CareerRowDto, CareerSectionDto, CareerStatsDto, CreatePlayerDto, DraftPickDto,
    EntityFieldProfileDto, EnumOptionDto, EquipmentZoneDto, ExportOptionsDto, ExtractSummaryDto,
    FieldCellDto, FieldSectionDto, FieldViewDto, FileExportOptionsDto, FileExportSummaryDto,
    FindingDto, HeadPreviewDto, ImportItemResultDto, InstallReportDto, IntegrityReportDto,
    KitPieceDto, KitPreviewDto, LineEntryDto, LinesViewDto, MatchCandidateDto, OpOutcomeDto,
    OutfitExportSummaryDto,
    PlayerAssetsDto, PlayerAttributesDto, PlayerDataDto, PlayerHeaderDto, PlayerSummaryDto,
    PlayerViewDto, PreviewSubmeshDto, PreviewTextureDto, ProjectSummaryDto, RosterMembershipDto,
    SaveSlotDto, SlotFieldDto, StrategyCellDto, SurfaceCapabilitiesDto, SurfaceCapabilityDto,
    TableCapacityDto, TableSummaryDto, TeamEquipmentDto, TeamJerseyDto, TeamStrategiesDto,
    TeamSummaryDto, TeamViewDto, TraitOptionDto, TraitViewDto,
};

/// Map an install report to its DTO, attaching platform-aware display labels to each save slot and
/// flagging whether a sidecar profile already exists in the install dir.
pub fn install_report_dto(report: &InstallReport, has_existing_profile: bool) -> InstallReportDto {
    InstallReportDto {
        game_id: report.game_id.clone(),
        platform: report.platform.clone(),
        picked_root: report.picked_root.clone(),
        install_dir: report.install_dir.clone(),
        nhlng_path: report.db_paths.nhlng.clone(),
        localization_path: report.db_paths.localization.clone(),
        meta_xml_path: report.db_paths.meta_xml.clone(),
        saves: report
            .saves
            .iter()
            .map(|s| save_slot_dto(s, &report.platform))
            .collect(),
        minimum_met: report.minimum_met,
        extract_needed: report.extract_needed,
        has_existing_profile,
    }
}

/// Map an extraction summary to its DTO.
pub fn extract_summary_dto(s: &ExtractSummary) -> ExtractSummaryDto {
    ExtractSummaryDto {
        archives: s.archives,
        files_written: s.files_written,
        bytes_written: s.bytes_written,
        skipped_unsupported: s.skipped_unsupported,
        skipped_unsafe: s.skipped_unsafe,
        archives_failed: s.archives_failed,
    }
}

fn save_slot_dto(slot: &SaveSlot, platform: &str) -> SaveSlotDto {
    SaveSlotDto {
        kind: slot.kind.clone(),
        label: save_label(&slot.kind, platform).to_string(),
        path: slot.path.clone(),
        container_version: slot.container_version.clone(),
        writable: slot.writable,
    }
}

/// Display label for a save kind. On Xbox the settings live inside the Profile save (no separate
/// settings save), so that slot is labelled accordingly.
fn save_label(kind: &str, platform: &str) -> &'static str {
    match kind {
        "settings" if platform == "xbox" => "Profile (incl. settings)",
        "settings" => "Settings",
        "roster" => "Roster",
        "beagm" => "Be A GM",
        "beapro" => "Be A Pro",
        "hockeycard" => "Hockey Card",
        "season" => "Season",
        _ => "Unknown",
    }
}

/// Build the project summary DTO for a workspace (reflects the *displayed* working db — canonical
/// `nhlng.db` unless the roster display toggle is on — incl. any growth, plus the roster-mirror
/// status). `dirty`/`undo_depth`/`redo_depth` are workspace-level.
pub fn project_summary_dto(
    project_id: u32,
    ws: &Workspace,
) -> Result<ProjectSummaryDto, studio_domain::DomainError> {
    let session = ws.display();
    let overview = session.overview()?;
    let project = session.project();
    let capabilities = capabilities_dto(&session.capabilities()?);
    Ok(ProjectSummaryDto {
        project_id,
        game_id: ws.game_id().map(str::to_string),
        platform: project.platform.as_str().to_string(),
        role: project.role.as_str().to_string(),
        source: source_str(project.source()).to_string(),
        endian: format!("{:?}", overview.endian),
        table_count: overview.table_count,
        db_size: overview.db_size,
        header_crc_ok: overview.header_crc_ok,
        size_ok: overview.size_ok,
        dirty: ws.is_dirty(),
        undo_depth: ws.undo_depth(),
        redo_depth: ws.redo_depth(),
        tables: table_dtos(&overview),
        roster_attached: ws.roster_attached(),
        roster_aligned: ws.roster_aligned(),
        display_roster: ws.display_roster(),
        capabilities,
        surfaces: surface_caps_dto(&session.surface_capabilities()?),
        // Experimental if either the canonical session or the (possibly v1) roster mirror writes
        // through an un-reversed integrity tag.
        write_experimental: write_experimental(project.source())
            || ws.roster_source().is_some_and(write_experimental),
    })
}

/// Whether this project's on-disk write path is only experimentally supported: PS4 roster saves
/// (repack unverified on console) or v1 containers (un-reversed `tag_1c`). Raw dbs and v2/v4/gen6
/// containers are fully supported.
fn write_experimental(source: &Source) -> bool {
    match source {
        Source::RawDb => false,
        Source::Container { original } => tdb_savedata::SaveContainer::parse(original)
            .map(|c| c.header.version.write_is_experimental())
            .unwrap_or(false),
        Source::Ps4Save { .. } => true,
    }
}

fn capabilities_dto(c: &Capabilities) -> CapabilitiesDto {
    CapabilitiesDto {
        arena_link: c.arena_link,
        player_style: c.player_style,
        year_ufa: c.year_ufa,
        junior_team: c.junior_team,
        draft_team: c.draft_team,
        draft_year: c.draft_year,
        draft_round: c.draft_round,
        draft_pick: c.draft_pick,
        rookie_ahl: c.rookie_ahl,
        rookie_chl: c.rookie_chl,
        has_skater_equip: c.has_skater_equip,
        has_goalie_equip: c.has_goalie_equip,
        skater_ratings_labelled: c.skater_ratings_labelled,
        skater_ratings_total: c.skater_ratings_total,
        goalie_ratings_labelled: c.goalie_ratings_labelled,
        goalie_ratings_total: c.goalie_ratings_total,
        main_team_offset: c.main_team_offset,
    }
}

fn surface_state_str(s: SurfaceState) -> &'static str {
    match s {
        SurfaceState::NotApplicable => "not-applicable",
        SurfaceState::Readable => "readable",
        SurfaceState::Editable => "editable",
        SurfaceState::Disabled => "disabled",
        SurfaceState::Unverified => "unverified",
    }
}

fn surface_cap_dto(c: SurfaceCapability) -> SurfaceCapabilityDto {
    SurfaceCapabilityDto {
        state: surface_state_str(c.state).to_string(),
        reason: c.reason.map(str::to_string),
    }
}

fn surface_caps_dto(s: &SurfaceCapabilities) -> SurfaceCapabilitiesDto {
    SurfaceCapabilitiesDto {
        players: surface_cap_dto(s.players),
        teams: surface_cap_dto(s.teams),
        arenas: surface_cap_dto(s.arenas),
        career: surface_cap_dto(s.career),
        attributes: surface_cap_dto(s.attributes),
        traits: surface_cap_dto(s.traits),
        lines: surface_cap_dto(s.lines),
        strategies: surface_cap_dto(s.strategies),
        captaincy: surface_cap_dto(s.captaincy),
        contracts: surface_cap_dto(s.contracts),
        draft_picks: surface_cap_dto(s.draft_picks),
        jerseys: surface_cap_dto(s.jerseys),
        team_equipment: surface_cap_dto(s.team_equipment),
        player_equipment: surface_cap_dto(s.player_equipment),
        portraits: surface_cap_dto(s.portraits),
        logos: surface_cap_dto(s.logos),
        heads: surface_cap_dto(s.heads),
        audio: surface_cap_dto(s.audio),
    }
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

/// Map a humanized Layer 4 report to its DTO (with per-severity tallies precomputed for the UI).
pub fn report_dto(report: &HumanReport) -> IntegrityReportDto {
    IntegrityReportDto {
        clean: report.is_clean(),
        error_count: report.count(Severity::Error),
        warning_count: report.count(Severity::Warning),
        info_count: report.count(Severity::Info),
        findings: report.findings.iter().map(finding_dto).collect(),
    }
}

fn finding_dto(f: &HumanFinding) -> FindingDto {
    FindingDto {
        severity: severity_str(f.severity).to_string(),
        title: f.title.clone(),
        location: f.location.clone(),
        message: f.message.clone(),
        predicate: f.predicate.to_string(),
        technical: f.technical.clone(),
        table: f.studio_table.clone(),
        field: f.studio_field.clone(),
        record: f.record,
    }
}

// ---- Roster Builder import (Phase 5.4) --------------------------------------

/// Map a FE cart line to the domain [`ImportItem`]: a `null` match id ⇒ CreateNew, else Match(id).
pub fn import_item_from_dto(dto: &crate::dto::ImportItemDto) -> ImportItem {
    ImportItem {
        person_id: dto.person_id,
        season_year: dto.season_year,
        target_team_id: dto.target_team_id,
        identity: match dto.match_ea_player_id {
            Some(id) => ImportIdentity::Match(id),
            None => ImportIdentity::CreateNew,
        },
        jersey: dto.jersey,
    }
}

/// An EA match candidate → DTO (confidence lowered to a stable wire string).
pub fn match_candidate_dto(c: &MatchCandidate) -> MatchCandidateDto {
    MatchCandidateDto {
        ea_player_id: c.ea_player_id,
        full_name: c.full_name.clone(),
        dob: c.dob.clone(),
        team_name: c.team_name.clone(),
        confidence: match c.confidence {
            MatchConfidence::NameAndDob => "name_and_dob",
            MatchConfidence::NameOnly => "name_only",
        }
        .to_string(),
        ambiguous: c.ambiguous,
    }
}

/// A projected roster-table capacity → DTO.
pub fn table_capacity_dto(c: &TableCapacity) -> TableCapacityDto {
    TableCapacityDto {
        table: c.table.clone(),
        label: c.label.clone(),
        current: c.current,
        ceiling: c.ceiling,
        adding: c.adding,
        over: c.over,
        enforced: c.enforced,
    }
}

/// A lowered cart item → its result row. `already_imported` is the ledger dedup flag, supplied by the
/// caller (the registry, which owns the ledger).
pub fn lowered_item_result_dto(item: &LoweredItem, already_imported: bool) -> ImportItemResultDto {
    let (outcome, ea_player_id) = match item.outcome {
        ItemOutcome::Created => ("created", None),
        ItemOutcome::Matched(id) => ("matched", Some(id)),
        ItemOutcome::PersonMissing => ("person_missing", None),
        ItemOutcome::NoSuchPlayer(id) => ("no_such_player", Some(id)),
        ItemOutcome::NoBioTable => ("no_bio_table", None),
    };
    ImportItemResultDto {
        person_id: item.person_id,
        season_year: item.season_year,
        target_team_id: item.target_team_id,
        full_name: item.full_name.clone(),
        outcome: outcome.to_string(),
        ea_player_id,
        skipped_fields: item.skipped.iter().map(|s| s.field.clone()).collect(),
        already_imported,
        lowered: item.lowered(),
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
        enum_options: fv
            .enum_options
            .iter()
            .map(|(raw, label)| EnumOptionDto {
                raw: *raw,
                label: label.clone(),
            })
            .collect(),
        edit_offset: fv.edit_offset,
        edit_height: fv.edit_height,
        edit_scale: fv.edit_scale,
        raw_int,
        text,
        range_low: fv.range_low,
        range_high: fv.range_high,
        editable: fv.editable,
        is_string: fv.is_string,
    }
}

/// Map one curated profile cell to its DTO, flattening the control hint into the wire fields.
pub fn field_cell_dto(c: &ProfileCell) -> FieldCellDto {
    let (control, team_offset, none_label) = match &c.control {
        CellControl::Field => ("field", None, None),
        CellControl::Team { offset, none_label } => ("team", Some(*offset), none_label.clone()),
        CellControl::Arena => ("arena", None, None),
        CellControl::Derived => ("derived", None, None),
    };
    FieldCellDto {
        label: c.label.clone(),
        control: control.to_string(),
        field: c.field.as_ref().map(field_view_dto),
        record: c.record,
        team_offset,
        none_label,
        derived_display: c.derived_display.clone(),
        unverified: c.unverified.clone(),
    }
}

/// Map a whole entity field profile (sections → cells) to its DTO.
pub fn entity_field_profile_dto(p: &EntityFieldProfile) -> EntityFieldProfileDto {
    EntityFieldProfileDto {
        sections: p
            .sections
            .iter()
            .map(|s| FieldSectionDto {
                title: s.title.clone(),
                cells: s.cells.iter().map(field_cell_dto).collect(),
            })
            .collect(),
    }
}

pub fn player_summary_dto(s: &PlayerSummary) -> PlayerSummaryDto {
    PlayerSummaryDto {
        player_id: s.player_id,
        first_name: s.first_name.clone(),
        last_name: s.last_name.clone(),
        position_label: s.position_label.clone(),
        team_id: s.team_id,
        jersey_number: s.jersey_number,
        style_label: s.style_label.clone(),
        is_active: s.is_active,
        overall: s.overall,
        rights_team_id: s.rights_team_id,
        bio_record: s.bio_record,
        cap_hit_dollars: s.cap_hit_dollars,
        contract_years: s.contract_years,
        contract_status: s.contract_status.clone(),
        is_two_way: s.is_two_way,
        is_goalie: s.is_goalie,
    }
}

fn membership_dto(m: &RosterMembership) -> RosterMembershipDto {
    RosterMembershipDto {
        team_id: m.team_id,
        team_name: m.team_name.clone(),
        jersey: m.jersey,
        captaincy: m.captaincy,
        dressed: m.dressed,
    }
}

fn player_header_dto(h: &PlayerHeader) -> PlayerHeaderDto {
    PlayerHeaderDto {
        full_name: h.full_name.clone(),
        position_label: h.position_label.clone(),
        handedness_label: h.handedness_label.clone(),
        nationality_label: h.nationality_label.clone(),
        birthplace: h.birthplace.clone(),
        birthdate: h.birthdate.clone(),
        height: h.height.clone(),
        weight: h.weight.clone(),
        style_label: h.style_label.clone(),
        primary_team_id: h.primary_team_id,
        primary_team_name: h.primary_team_name.clone(),
        jersey_number: h.jersey_number,
    }
}

fn player_data_dto(d: &PlayerData) -> PlayerDataDto {
    PlayerDataDto {
        year_ufa: d.year_ufa,
        pro_team_id: d.pro_team_id,
        pro_team_name: d.pro_team_name.clone(),
        cap_hit: d.cap_hit.clone(),
        contract_length_years: d.contract_length_years,
        contract_status: d.contract_status.clone(),
        is_two_way: d.is_two_way,
        is_multi_year: d.is_multi_year,
        junior_team_id: d.junior_team_id,
        junior_team_name: d.junior_team_name.clone(),
        draft_team_id: d.draft_team_id,
        draft_team_name: d.draft_team_name.clone(),
        draft_round: d.draft_round,
        draft_position: d.draft_position,
        draft_year: d.draft_year,
        is_rookie_nhl: d.is_rookie_nhl,
        is_rookie_ahl: d.is_rookie_ahl,
        is_rookie_chl: d.is_rookie_chl,
    }
}

fn attribute_view_dto(a: &AttributeView) -> AttributeViewDto {
    AttributeViewDto {
        label: a.label.clone(),
        is_rating: a.is_rating,
        field: a.field.as_ref().map(field_view_dto),
    }
}

fn trait_view_dto(t: &TraitView) -> TraitViewDto {
    TraitViewDto {
        slot_field: t.slot_field.clone(),
        slot_index: t.slot_index,
        value: t.value,
        label: t.label.clone(),
        confidence: t.confidence.to_string(),
        note: t.note.clone(),
    }
}

fn trait_option_dto(o: &TraitOption) -> TraitOptionDto {
    TraitOptionDto {
        value: o.value,
        label: o.label.clone(),
        confidence: o.confidence.to_string(),
        note: o.note.clone(),
        active: o.active,
    }
}

fn player_attributes_dto(a: &PlayerAttributes) -> PlayerAttributesDto {
    PlayerAttributesDto {
        kind: a.kind.map(|k| k.to_string()),
        overall: a.overall.as_ref().map(attribute_view_dto),
        style: a.style.as_ref().map(attribute_view_dto),
        potential_range: a.potential_range.as_ref().map(attribute_view_dto),
        potential_accuracy: a.potential_accuracy.as_ref().map(attribute_view_dto),
        flat: a.flat.iter().map(attribute_view_dto).collect(),
        traits: a.traits.iter().map(trait_view_dto).collect(),
        trait_options: a.trait_options.iter().map(trait_option_dto).collect(),
    }
}

pub fn player_view_dto(v: &PlayerView) -> PlayerViewDto {
    PlayerViewDto {
        player_id: v.player_id,
        bio_record: v.bio_record,
        kind: v.kind.to_string(),
        header: player_header_dto(&v.header),
        data: player_data_dto(&v.data),
        attributes: player_attributes_dto(&v.attributes),
        identity: v.identity.iter().map(field_view_dto).collect(),
        ratings: v.ratings.iter().map(field_view_dto).collect(),
        ratings_record: v.ratings_record,
        ratings_table: v.ratings_table.to_string(),
        equipment: v.equipment.iter().map(field_view_dto).collect(),
        equipment_record: v.equipment_record,
        equipment_table: v.equipment_table.to_string(),
        memberships: v.memberships.iter().map(membership_dto).collect(),
        all_fields: v.all_fields.iter().map(field_view_dto).collect(),
        profile: entity_field_profile_dto(&v.profile),
    }
}

pub fn career_stats_dto(c: &CareerStats) -> CareerStatsDto {
    CareerStatsDto {
        sections: c
            .sections
            .iter()
            .map(|s| CareerSectionDto {
                kind: s.kind.clone(),
                title: s.title.clone(),
                table: s.table.clone(),
                columns: s
                    .columns
                    .iter()
                    .map(|col| CareerColumnDto {
                        field: col.field.clone(),
                        label: col.label.clone(),
                    })
                    .collect(),
                rows: s
                    .rows
                    .iter()
                    .map(|r| CareerRowDto {
                        record: r.record,
                        season_label: r.season_label.clone(),
                        league_label: r.league_label.clone(),
                        team: r.team.clone(),
                        cells: r
                            .cells
                            .iter()
                            .map(|c| c.as_ref().map(field_view_dto))
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn line_entry_dto(e: &LineEntry) -> LineEntryDto {
    LineEntryDto {
        slot_record: e.slot_record,
        player_id: e.player_id,
        name: e.name.clone(),
        position_label: e.position_label.clone(),
        jersey: e.jersey,
        captaincy: e.captaincy,
        dressed: e.dressed,
        slots: e.slots.clone(),
        style: e.style.as_ref().map(field_view_dto),
    }
}

pub fn lines_view_dto(v: &LinesView) -> LinesViewDto {
    LinesViewDto {
        team_id: v.team_id,
        slot_fields: v
            .slot_fields
            .iter()
            .map(|s| SlotFieldDto {
                studio_field: s.studio_field.clone(),
                name: s.name.clone(),
            })
            .collect(),
        entries: v.entries.iter().map(line_entry_dto).collect(),
    }
}

pub fn draft_pick_dto(p: &studio_domain::DraftPick) -> DraftPickDto {
    DraftPickDto {
        record: p.record,
        year: p.year,
        round: p.round,
        overall: p.overall,
        original_owner_team_id: p.original_owner_team_id,
        original_owner_name: p.original_owner_name.clone(),
        via_trade: p.via_trade,
    }
}

fn strategy_cell_dto(c: &studio_domain::StrategyCell) -> StrategyCellDto {
    StrategyCellDto {
        label: c.label.clone(),
        field: field_view_dto(&c.field),
    }
}

pub fn team_strategies_dto(s: &studio_domain::TeamStrategies) -> TeamStrategiesDto {
    TeamStrategiesDto {
        team_id: s.team_id,
        record: s.record,
        tactics: s.tactics.iter().map(strategy_cell_dto).collect(),
        forward_biases: s.forward_biases.iter().map(strategy_cell_dto).collect(),
        defense_biases: s.defense_biases.iter().map(strategy_cell_dto).collect(),
        team_biases: s.team_biases.iter().map(strategy_cell_dto).collect(),
    }
}

pub fn team_jersey_dto(j: &studio_domain::TeamJersey) -> TeamJerseyDto {
    TeamJerseyDto {
        record: j.record,
        jersey_index: j.jersey_index,
        style_raw: j.style_raw,
        team_art_id: j.team_art_id,
        variant_raw: j.variant_raw,
        fe_variant_raw: j.fe_variant_raw,
        helmet_logo: j.helmet_logo,
        helmet_numbers: j.helmet_numbers,
        is_light_color: j.is_light_color,
        has_tie: j.has_tie,
        loc_string_raw: j.loc_string_raw,
        is_default: j.is_default,
        is_lockable: j.is_lockable,
    }
}

pub fn team_equipment_dto(e: &studio_domain::TeamEquipment) -> TeamEquipmentDto {
    TeamEquipmentDto {
        record: e.record,
        equipment_id: e.equipment_id,
        equipment_type_raw: e.equipment_type_raw,
        brand_raw: e.brand_raw,
        era_raw: e.era_raw,
        variant_raw: e.variant_raw,
        zones: e
            .zones
            .iter()
            .map(|z| EquipmentZoneDto {
                red: z.red,
                green: z.green,
                blue: z.blue,
            })
            .collect(),
    }
}

pub fn team_summary_dto(t: &TeamSummary) -> TeamSummaryDto {
    TeamSummaryDto {
        team_id: t.team_id,
        record: t.record,
        name: t.name.clone(),
        abbrev: t.abbrev.clone(),
        league: t.league,
        league_label: t.league_label.clone(),
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
        retired_numbers: v.retired_numbers.clone(),
        all_fields: v.all_fields.iter().map(field_view_dto).collect(),
        profile: entity_field_profile_dto(&v.profile),
    }
}

pub fn arena_summary_dto(a: &ArenaSummary) -> ArenaSummaryDto {
    ArenaSummaryDto {
        arena_id: a.arena_id,
        record: a.record,
        name: a.name.clone(),
        city: a.city.clone(),
        team_count: a.team_count,
    }
}

pub fn arena_view_dto(v: &ArenaView) -> ArenaViewDto {
    ArenaViewDto {
        arena_id: v.arena_id,
        record: v.record,
        identity: v.identity.iter().map(field_view_dto).collect(),
        teams: v.teams.iter().map(team_summary_dto).collect(),
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
        Source::Ps4Save { .. } => "ps4_save",
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

// ---- Historical (FHM corpus) converters -----------------------------------

use crate::dto::{
    HistoricalAwardDto, HistoricalCareerRowDto, HistoricalContractDto, HistoricalDraftDto,
    HistoricalFiltersDto, HistoricalLeagueOptionDto, HistoricalPersonDto,
    HistoricalPersonSummaryDto, HistoricalRosterEntryDto, HistoricalSeasonOptionDto,
    HistoricalSeasonRowDto, HistoricalStatCellDto, HistoricalTeamAffiliateDto,
    HistoricalTeamBlockDto, HistoricalTeamContractDto, HistoricalTeamDto, HistoricalTeamFiltersDto,
    HistoricalTeamFranchiseDto, HistoricalTeamIndexEntryDto, HistoricalTeamMemberDto,
    HistoricalTeamSummaryDto,
};
use studio_domain::historical::{
    AwardRow, CareerStatRow, ContractRow, DraftRow, LeagueOption, PersonDetail, PersonSummary,
    PlayerSeasonRow, SeasonOption, TeamAffiliate, TeamContractLine, TeamFranchiseDetail,
    TeamIndexEntry, TeamRosterEntry, TeamRosterMember, TeamSeasonBlock, TeamSeasonDetail,
    TeamSeasonSummary,
};

/// Curated career-column display order (matches the FHM export layout); extras are appended after.
const SKATER_COLS: &[&str] = &[
    "GP",
    "G",
    "A",
    "PIM",
    "+/-",
    "PP G",
    "PP A",
    "SH G",
    "SH A",
    "GWG",
    "SOG",
    "GR",
    "FO",
    "FO W",
    "HIT",
    "GvA",
    "TkA",
    "SB",
    "TOI",
    "PPTOI",
    "SHTOI",
    "Fights",
    "Fights Won",
];
const GOALIE_COLS: &[&str] = &[
    "GP", "GS", "Min", "W", "L", "T/OL", "ENG", "SO", "GA", "SA", "GR",
];

fn season_label(year: i64) -> String {
    format!("{}-{}", year - 1, year)
}

pub fn historical_filters_dto(
    seasons: &[SeasonOption],
    leagues: &[LeagueOption],
) -> HistoricalFiltersDto {
    HistoricalFiltersDto {
        seasons: seasons
            .iter()
            .map(|s| HistoricalSeasonOptionDto {
                year: s.year,
                label: s.label.clone(),
            })
            .collect(),
        leagues: leagues
            .iter()
            .map(|l| HistoricalLeagueOptionDto {
                abbr: l.abbr.clone(),
                name: l.name.clone(),
            })
            .collect(),
    }
}

pub fn historical_person_summary_dto(p: &PersonSummary) -> HistoricalPersonSummaryDto {
    HistoricalPersonSummaryDto {
        person_id: p.person_id,
        first_name: p.first_name.clone(),
        last_name: p.last_name.clone(),
        dob: p.dob.clone(),
        nationality: p.nationality.clone(),
        is_goalie: p.is_goalie,
        position: p.position.clone(),
        first_year: p.first_year,
        last_year: p.last_year,
        season_count: p.season_count,
        retired: p.retired,
        active: p.active,
        seasons_played: p.seasons_played,
    }
}

/// Build the person-detail DTO. `current_season` is the corpus's newest season, used to decide
/// whether a still-active player's career end should read "Present". The career end is derived from
/// the games-played-filtered `seasons` (newest first), not the raw last appearance, so phantom
/// post-retirement re-listings don't inflate it.
pub fn historical_person_dto(p: &PersonDetail, current_season: Option<i64>) -> HistoricalPersonDto {
    let last_played = p.seasons.iter().map(|s| s.season_year).max();
    let active = last_played.is_some() && last_played == current_season;
    HistoricalPersonDto {
        person_id: p.person_id,
        first_name: p.first_name.clone(),
        last_name: p.last_name.clone(),
        nick_name: p.nick_name.clone(),
        dob: p.dob.clone(),
        height: p.height,
        weight: p.weight,
        hand: p.hand.clone(),
        nationality: p.nationality.clone(),
        birthcity: p.birthcity.clone(),
        birthstate: p.birthstate.clone(),
        retired: p.retired,
        is_goalie: p.is_goalie,
        position: p.position.clone(),
        first_year: p.first_year,
        last_year: last_played.or(p.last_year),
        active,
        seasons: p.seasons.iter().map(historical_season_row_dto).collect(),
        career: p.career.iter().map(historical_career_row_dto).collect(),
        drafts: p.drafts.iter().map(historical_draft_dto).collect(),
        contract: p.contract.as_ref().map(historical_contract_dto),
        awards: p.awards.iter().map(historical_award_dto).collect(),
    }
}

fn historical_contract_dto(c: &ContractRow) -> HistoricalContractDto {
    HistoricalContractDto {
        team_abbr: c.team_abbr.clone(),
        ntc: c.ntc,
        nmc: c.nmc,
        elc: c.elc,
        ufa: c.ufa,
        scholarship: c.scholarship.clone(),
        avg_salary: c.avg_salary,
        length: c.length,
    }
}

fn historical_season_row_dto(s: &PlayerSeasonRow) -> HistoricalSeasonRowDto {
    HistoricalSeasonRowDto {
        season_year: s.season_year,
        season_label: season_label(s.season_year),
        team_name: s.team_name.clone(),
        team_abbr: s.team_abbr.clone(),
        league_abbr: s.league_abbr.clone(),
        height: s.height,
        weight: s.weight,
        jersey: s.jersey,
        ability: s.ability,
        potential: s.potential,
    }
}

fn historical_career_row_dto(c: &CareerStatRow) -> HistoricalCareerRowDto {
    HistoricalCareerRowDto {
        kind: c.kind.clone(),
        phase: c.phase.clone(),
        year: c.year,
        team_abbr: c.team_abbr.clone(),
        league_abbr: c.league_abbr.clone(),
        cells: stat_cells(&c.kind, &c.stats_json),
    }
}

/// Parse a career row's `stats_json` into display cells, ordered by the curated column list for the
/// player kind; any keys not in the list are appended (sorted) so nothing is silently dropped.
fn stat_cells(kind: &str, stats_json: &str) -> Vec<HistoricalStatCellDto> {
    let map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(stats_json).unwrap_or_default();
    let order: &[&str] = if kind == "goalie" {
        GOALIE_COLS
    } else {
        SKATER_COLS
    };
    let mut cells = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &label in order {
        if let Some(v) = map.get(label) {
            cells.push(HistoricalStatCellDto {
                label: label.to_string(),
                value: json_value_str(v),
            });
            seen.insert(label.to_string());
        }
    }
    let mut extras: Vec<&String> = map.keys().filter(|k| !seen.contains(*k)).collect();
    extras.sort();
    for k in extras {
        cells.push(HistoricalStatCellDto {
            label: k.clone(),
            value: json_value_str(&map[k]),
        });
    }
    cells
}

fn json_value_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn historical_draft_dto(d: &DraftRow) -> HistoricalDraftDto {
    HistoricalDraftDto {
        year: d.year,
        round: d.round,
        pick: d.pick,
        overall: d.overall,
        team: d.team.clone(),
        draft_name: d.draft_name.clone(),
        is_nhl: d.is_nhl,
    }
}

fn historical_award_dto(a: &AwardRow) -> HistoricalAwardDto {
    HistoricalAwardDto {
        year: a.year,
        league_abbr: a.league_abbr.clone(),
        name: a.name.clone(),
        description: a.description.clone(),
    }
}

pub fn historical_team_summary_dto(t: &TeamSeasonSummary) -> HistoricalTeamSummaryDto {
    HistoricalTeamSummaryDto {
        season_year: t.season_year,
        src_team_id: t.src_team_id,
        league_abbr: t.league_abbr.clone(),
        name: t.name.clone(),
        nickname: t.nickname.clone(),
        abbr: t.abbr.clone(),
    }
}

pub fn historical_team_dto(t: &TeamSeasonDetail) -> HistoricalTeamDto {
    HistoricalTeamDto {
        season_year: t.season_year,
        src_team_id: t.src_team_id,
        name: t.name.clone(),
        nickname: t.nickname.clone(),
        abbr: t.abbr.clone(),
        league_abbr: t.league_abbr.clone(),
        conference_name: t.conference_name.clone(),
        division_name: t.division_name.clone(),
        primary_color: t.primary_color.clone(),
        secondary_color: t.secondary_color.clone(),
        roster: t.roster.iter().map(historical_roster_entry_dto).collect(),
    }
}

fn historical_roster_entry_dto(e: &TeamRosterEntry) -> HistoricalRosterEntryDto {
    HistoricalRosterEntryDto {
        person_id: e.person_id,
        first_name: e.first_name.clone(),
        last_name: e.last_name.clone(),
        is_goalie: e.is_goalie,
        ability: e.ability,
        potential: e.potential,
    }
}

// ---- Franchise-consolidated team browse (v5) ------------------------------

pub fn historical_team_filters_dto(leagues: &[LeagueOption]) -> HistoricalTeamFiltersDto {
    HistoricalTeamFiltersDto {
        leagues: leagues
            .iter()
            .map(|l| HistoricalLeagueOptionDto {
                abbr: l.abbr.clone(),
                name: l.name.clone(),
            })
            .collect(),
    }
}

pub fn historical_team_index_entry_dto(e: &TeamIndexEntry) -> HistoricalTeamIndexEntryDto {
    HistoricalTeamIndexEntryDto {
        kind: e.kind.clone(),
        key: e.key.clone(),
        name: e.name.clone(),
        abbr: e.abbr.clone(),
        league_abbr: e.league_abbr.clone(),
        first_year: e.first_year,
        last_year: e.last_year,
        is_defunct: e.is_defunct,
    }
}

pub fn historical_team_franchise_dto(d: &TeamFranchiseDetail) -> HistoricalTeamFranchiseDto {
    HistoricalTeamFranchiseDto {
        kind: d.kind.clone(),
        key: d.key.clone(),
        display_name: d.display_name.clone(),
        abbr: d.abbr.clone(),
        league_abbr: d.league_abbr.clone(),
        is_defunct: d.is_defunct,
        notes: d.notes.clone(),
        seasons: d.seasons.clone(),
        season_year: d.season_year,
        constituents: d
            .constituents
            .iter()
            .map(historical_team_block_dto)
            .collect(),
    }
}

fn historical_team_block_dto(b: &TeamSeasonBlock) -> HistoricalTeamBlockDto {
    HistoricalTeamBlockDto {
        team_key: b.team_key.clone(),
        src_team_id: b.src_team_id,
        season_year: b.season_year,
        name: b.name.clone(),
        nickname: b.nickname.clone(),
        abbr: b.abbr.clone(),
        league_abbr: b.league_abbr.clone(),
        conference_name: b.conference_name.clone(),
        division_name: b.division_name.clone(),
        primary_color: b.primary_color.clone(),
        secondary_color: b.secondary_color.clone(),
        text_color: b.text_color.clone(),
        affiliates: b
            .affiliates
            .iter()
            .map(historical_team_affiliate_dto)
            .collect(),
        roster: b.roster.iter().map(historical_team_member_dto).collect(),
        contracts: b
            .contracts
            .iter()
            .map(historical_team_contract_dto)
            .collect(),
    }
}

fn historical_team_affiliate_dto(a: &TeamAffiliate) -> HistoricalTeamAffiliateDto {
    HistoricalTeamAffiliateDto {
        relation: a.relation.clone(),
        src_team_id: a.src_team_id,
        name: a.name.clone(),
        abbr: a.abbr.clone(),
        league_abbr: a.league_abbr.clone(),
    }
}

fn historical_team_member_dto(m: &TeamRosterMember) -> HistoricalTeamMemberDto {
    HistoricalTeamMemberDto {
        person_id: m.person_id,
        first_name: m.first_name.clone(),
        last_name: m.last_name.clone(),
        is_goalie: m.is_goalie,
        position: m.position.clone(),
        dob: m.dob.clone(),
        nationality: m.nationality.clone(),
        jersey: m.jersey,
        captaincy: m.captaincy.clone(),
        ability: m.ability,
        potential: m.potential,
    }
}

fn historical_team_contract_dto(c: &TeamContractLine) -> HistoricalTeamContractDto {
    HistoricalTeamContractDto {
        person_id: c.person_id,
        first_name: c.first_name.clone(),
        last_name: c.last_name.clone(),
        avg_salary: c.avg_salary,
        ntc: c.ntc,
        nmc: c.nmc,
        elc: c.elc,
        ufa: c.ufa,
    }
}

// ---- Formula engine (Stage 2) converters ----------------------------------

use crate::dto::{
    CareerSeasonOverallDto, EaComputedRatingsDto, EaRatingDto, FhmAttrDto, FhmGroupDto,
    FhmValueDto, FormulaCatalogDto, FormulaProfileDto, HistoricalCareerRatingsDto, PrebuiltDto,
};
use studio_domain::formula::catalog as fcat;
use studio_domain::{CareerRatings, ComputedRatings, FormulaProfile};

/// Map a computed EA rating set (+ its raw FHM inputs) → wire DTO.
pub fn ea_computed_ratings_dto(cr: &ComputedRatings) -> EaComputedRatingsDto {
    let set = &cr.set;
    EaComputedRatingsDto {
        position: set.position.clone(),
        overall: set.overall,
        style_label: set.archetype.as_ref().map(|a| a.ea_label.clone()),
        style_raw: set.archetype.as_ref().map(|a| a.ea_raw),
        ratings: set
            .ratings
            .iter()
            .map(|r| EaRatingDto {
                ea_key: r.ea_key.clone(),
                value: r.value,
                scale_min: r.scale_min,
                scale_max: r.scale_max,
            })
            .collect(),
        fhm_values: cr
            .fhm_values
            .iter()
            .map(|(k, v)| FhmValueDto {
                key: k.clone(),
                value: *v,
            })
            .collect(),
    }
}

/// Cross-career EA rating compilations → wire DTO.
pub fn historical_career_ratings_dto(cr: &CareerRatings) -> HistoricalCareerRatingsDto {
    HistoricalCareerRatingsDto {
        position: cr.position.clone(),
        seasons: cr
            .seasons
            .iter()
            .map(|s| CareerSeasonOverallDto {
                season_year: s.season_year,
                overall: s.overall,
            })
            .collect(),
        best_season_year: cr.best_season_year,
        career_average: cr.career_average.as_ref().map(ea_computed_ratings_dto),
        peak3: cr.peak3.as_ref().map(ea_computed_ratings_dto),
    }
}

/// Domain profile → DTO (serde round-trip; the shapes are identical).
pub fn formula_profile_dto(p: &FormulaProfile) -> FormulaProfileDto {
    serde_json::from_value(serde_json::to_value(p).unwrap_or_default()).unwrap_or_else(|_| {
        FormulaProfileDto {
            forward: Default::default(),
            defenseman: Default::default(),
            goalie: Default::default(),
            normalisation_mode: "nominal".to_string(),
        }
    })
}

/// DTO → domain profile (serde round-trip; unknown modes fall back to Nominal).
pub fn formula_profile_from_dto(dto: &FormulaProfileDto) -> FormulaProfile {
    serde_json::from_value(serde_json::to_value(dto).unwrap_or_default()).unwrap_or_default()
}

/// The static formula catalog for the Formulas UI.
pub fn formula_catalog_dto() -> FormulaCatalogDto {
    FormulaCatalogDto {
        skater_ea_attrs: fcat::SKATER_EA_ATTRS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        goalie_ea_attrs: fcat::GOALIE_EA_ATTRS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        fhm_groups: fcat::FHM_ATTR_GROUPS
            .iter()
            .map(|(group, attrs)| FhmGroupDto {
                group: (*group).to_string(),
                attrs: attrs
                    .iter()
                    .map(|(label, key)| FhmAttrDto {
                        key: (*key).to_string(),
                        label: (*label).to_string(),
                    })
                    .collect(),
            })
            .collect(),
        prebuilts: fcat::PREBUILT_META
            .iter()
            .map(|(key, label, native_max, direct)| PrebuiltDto {
                key: format!("{}{}", fcat::PREBUILT_PREFIX, key),
                label: (*label).to_string(),
                native_max: *native_max,
                direct_output: *direct,
            })
            .collect(),
    }
}

pub fn asset_roots_dto(r: &AssetRoots) -> AssetRootsDto {
    AssetRootsDto {
        rendering: r.rendering.clone(),
        fe: r.fe.clone(),
        complete: r.complete(),
    }
}

pub fn player_assets_dto(p: &PlayerAssets) -> PlayerAssetsDto {
    PlayerAssetsDto {
        bio_record: p.bio_record,
        full_name: p.full_name.clone(),
        slug: p.slug.clone(),
        portrait_id: p.portrait_id,
        head_id: p.head_id,
        bespoke_head: p.bespoke_head,
        pool_head: p.pool_head,
        portrait_path: p.portrait_path.clone(),
        mesh_path: p.mesh_path.clone(),
        texture_paths: p.texture_paths.clone(),
    }
}

pub fn export_options(o: &ExportOptionsDto) -> ExportOptions {
    ExportOptions {
        portraits: o.portraits,
        meshes_glb: o.meshes_glb,
        raw_rx2: o.raw_rx2,
        textures: o.textures,
        crop_portraits: o.crop_portraits,
    }
}

pub fn asset_export_summary_dto(s: &AssetExportSummary) -> AssetExportSummaryDto {
    AssetExportSummaryDto {
        players_considered: s.players_considered,
        portraits_written: s.portraits_written,
        meshes_written: s.meshes_written,
        textures_written: s.textures_written,
        raw_copied: s.raw_copied,
        portraits_skipped_generic: s.portraits_skipped_generic,
        skipped_no_slug: s.skipped_no_slug,
        failures: s.failures.clone(),
        bytes_written: s.bytes_written,
    }
}

pub fn browse_listing_dto(l: &BrowseListing) -> BrowseListingDto {
    BrowseListingDto {
        root: l.root.clone(),
        rel_path: l.rel_path.clone(),
        parent: l.parent.clone(),
        entries: l
            .entries
            .iter()
            .map(|e| BrowseEntryDto {
                name: e.name.clone(),
                rel_path: e.rel_path.clone(),
                is_dir: e.is_dir,
                size: e.size,
                kind: e.kind.map(|k| k.as_str().to_string()),
            })
            .collect(),
    }
}

pub fn file_export_options(o: &FileExportOptionsDto) -> FileExportOptions {
    FileExportOptions {
        meshes_glb: o.meshes_glb,
        portraits_png: o.portraits_png,
        textures: o.textures,
        copy_unrecognized: o.copy_unrecognized,
        crop_portraits: o.crop_portraits,
    }
}

pub fn file_export_summary_dto(s: &FileExportSummary) -> FileExportSummaryDto {
    FileExportSummaryDto {
        files_considered: s.files_considered,
        meshes_written: s.meshes_written,
        portraits_written: s.portraits_written,
        textures_written: s.textures_written,
        raw_copied: s.raw_copied,
        skipped_unrecognized: s.skipped_unrecognized,
        skipped_unsafe: s.skipped_unsafe,
        failures: s.failures.clone(),
        bytes_written: s.bytes_written,
    }
}

pub fn outfit_export_summary_dto(s: &OutfitExportSummary) -> OutfitExportSummaryDto {
    OutfitExportSummaryDto {
        teams_considered: s.teams_considered,
        jerseys_written: s.jerseys_written,
        pants_written: s.pants_written,
        socks_written: s.socks_written,
        failures: s.failures.clone(),
        bytes_written: s.bytes_written,
        rendering_root: s.rendering_root.clone(),
    }
}

/// Slugify a resolved team name for a filename: lowercase, every run of non-alphanumerics collapsed
/// to a single `_`, ends trimmed. "Detroit Red Wings" → `detroit_red_wings`. Empty when the name
/// has no usable characters (that team is then skipped by the exporter).
pub fn team_name_slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_sep = true; // trims leading separators
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

pub fn head_preview_dto(p: &HeadPreview) -> HeadPreviewDto {
    HeadPreviewDto {
        submeshes: p
            .submeshes
            .iter()
            .map(|s| PreviewSubmeshDto {
                name: s.name.clone(),
                positions: s.positions.clone(),
                normals: s.normals.clone(),
                uv1: s.uv1.clone(),
                triangles: s.triangles.clone(),
                material_index: s.material_index,
                transparent: s.transparent,
                render_order: s.render_order,
            })
            .collect(),
        textures: p
            .textures
            .iter()
            .map(|t| PreviewTextureDto {
                name: t.name.clone(),
                width: t.width,
                height: t.height,
                rgba_base64: t.rgba_base64.clone(),
            })
            .collect(),
        center: p.center,
        span: p.span,
        source_path: p.source_path.clone(),
        lenient_rig: p.lenient_rig,
    }
}

/// Map a decoded [`PreviewSubmesh`] to its DTO (shared by head + kit previews).
fn preview_submesh_dto(s: &studio_domain::PreviewSubmesh) -> PreviewSubmeshDto {
    PreviewSubmeshDto {
        name: s.name.clone(),
        positions: s.positions.clone(),
        normals: s.normals.clone(),
        uv1: s.uv1.clone(),
        triangles: s.triangles.clone(),
        material_index: s.material_index,
        transparent: s.transparent,
        render_order: s.render_order,
    }
}

/// Map a decoded [`PreviewTexture`] to its DTO (shared by head + kit previews).
fn preview_texture_dto(t: &studio_domain::PreviewTexture) -> PreviewTextureDto {
    PreviewTextureDto {
        name: t.name.clone(),
        width: t.width,
        height: t.height,
        rgba_base64: t.rgba_base64.clone(),
    }
}

pub fn kit_preview_dto(p: &studio_domain::KitPreview) -> KitPreviewDto {
    KitPreviewDto {
        pieces: p
            .pieces
            .iter()
            .map(|piece| KitPieceDto {
                kind: piece.kind.clone(),
                label: piece.label.clone(),
                source_path: piece.source_path.clone(),
                submeshes: piece.submeshes.iter().map(preview_submesh_dto).collect(),
            })
            .collect(),
        textures: p.textures.iter().map(preview_texture_dto).collect(),
        center: p.center,
        span: p.span,
    }
}

#[cfg(test)]
mod tests {
    use super::team_name_slug;

    #[test]
    fn team_name_slug_is_filename_safe() {
        assert_eq!(team_name_slug("Detroit Red Wings"), "detroit_red_wings");
        assert_eq!(team_name_slug("Montréal Canadiens"), "montr_al_canadiens"); // non-ascii dropped
        assert_eq!(team_name_slug("St. Louis Blues"), "st_louis_blues"); // punctuation collapses
        assert_eq!(team_name_slug("  Anaheim  Ducks  "), "anaheim_ducks"); // trimmed + collapsed
        assert_eq!(team_name_slug("---"), ""); // no usable chars → empty (team skipped)
    }
}
