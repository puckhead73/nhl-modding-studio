use super::Registry;
use crate::{convert, dto::ScheduleViewDto, error::ApiError};
use studio_domain::schedule_edit::{replacement_plan, GameRow};

impl Registry {
    pub fn swap_team_memberships(
        &self,
        id: u32,
        first: u64,
        second: u64,
    ) -> Result<crate::dto::OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(
                ws.swap_team_memberships(first, second)?,
            ))
        })
    }
    /// One shared fixture backs the calendars of both participants. Dates are 1-based here.
    pub fn team_schedule_edit(
        &self,
        id: u32,
        table: &str,
        team: u64,
        fingerprint: &str,
        record: Option<u32>,
        remove: bool,
        opponent: Option<u64>,
        home: bool,
        month: u64,
        day: u64,
        expected_games: &[GameRow],
    ) -> Result<ScheduleViewDto, ApiError> {
        self.with_workspace(id,|ws| {
            let model=ws.schedule_session().schedule_model(table)?;
            let fail=|s:&str| ApiError::Workspace(s.into());
            if model.slots.fingerprint != fingerprint || model.rows != expected_games { return Err(fail("The schedule changed. Reopen this day and try again.")); }
            let own=model.slots.by_team(team).ok_or_else(||fail("This team does not belong to this schedule."))?.slot;
            let mut rows=model.rows.clone();
            if let Some(r)=record {
                let row=rows.get(r as usize).ok_or_else(||fail("This game no longer exists."))?;
                if row.home_slot != own && row.away_slot != own { return Err(fail("This game does not involve the selected team.")); }
            } else if remove { return Err(fail("Choose a game to remove.")); }
            if remove { rows.remove(record.unwrap() as usize); }
            else {
                if !(1..=12).contains(&month) || !(1..=31).contains(&day) { return Err(fail("Choose a valid calendar date.")); }
                let other=model.slots.by_team(opponent.ok_or_else(||fail("Choose an opponent."))?).ok_or_else(||fail("Choose an opponent from this competition."))?.slot;
                if other==own { return Err(fail("A team cannot play itself.")); }
                // A second fixture is visible in the calendar, but new double bookings require correction.
                if rows.iter().enumerate().any(|(i,r)| Some(i as u32)!=record && r.month==month-1 && r.day==day-1 && [r.home_slot,r.away_slot].iter().any(|s| *s==own || *s==other)) {
                    return Err(fail("One of these teams already has a game on this date. Move or remove that game first."));
                }
                let old=record.and_then(|i|rows.get(i as usize));
                let row=GameRow{index:record.unwrap_or(rows.len() as u32) as u64, month:month-1,day:day-1,
                    home_slot:if home {own}else{other},away_slot:if home {other}else{own},round:old.map_or(0,|r|r.round),status:old.and_then(|r|r.status).or(Some(0))};
                if let Some(i)=record {rows[i as usize]=row;} else {rows.push(row);}
            }
            let plan=replacement_plan(&model,rows)?;
            if !plan.validation.ok() {return Err(ApiError::Workspace(plan.validation.errors.join("; ")));}
            if let Some(rows)=plan.replacement {ws.replace_schedules(&[(table.into(),rows)])?;}
            Ok(convert::schedule_view_dto(&ws.display().schedule_view(table)?.ok_or_else(||fail("Schedule unavailable."))?))
        })
    }
}
