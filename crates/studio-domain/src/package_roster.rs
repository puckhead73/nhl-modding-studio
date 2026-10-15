//! Explicit package roster-table growth. No row-position cell replay across an insertion.
use crate::{mod_package::RosterTableOp, DomainError, Result};
use std::collections::{BTreeMap, BTreeSet};
type Row = BTreeMap<String, u64>;
fn invalid(message: &str) -> DomainError {
    DomainError::Validation(message.into())
}

pub fn rows(bytes: &[u8]) -> Result<Vec<Row>> {
    table_rows(bytes, "eGlu")
}
fn table_rows(bytes: &[u8], name: &str) -> Result<Vec<Row>> {
    let db = tdb_core::TdbFile::parse(bytes).map_err(|e| invalid(&e.to_string()))?;
    let table = db
        .table_canonical(name)
        .ok_or_else(|| invalid("Roster identity table is missing"))?;
    (0..u32::from(table.current_records))
        .map(|r| {
            table
                .fields
                .iter()
                .map(|f| {
                    if f.bit_width > 64 {
                        return Err(invalid("Roster replacement requires integer fields"));
                    }
                    Ok((
                        f.id.canonical(db.endian),
                        db.read_u64(table, r, f)
                            .ok_or_else(|| invalid("Unreadable roster field"))?,
                    ))
                })
                .collect()
        })
        .collect()
}
fn links(bytes: &[u8]) -> Result<BTreeMap<u64, u64>> {
    let mut links = BTreeMap::new();
    for row in table_rows(bytes, "ZBac")? {
        let index = *row
            .get("vfEq")
            .ok_or_else(|| invalid("Missing player index"))?;
        let player = *row
            .get("ykFq")
            .ok_or_else(|| invalid("Missing player ID"))?;
        if links.insert(index, player).is_some() {
            return Err(invalid("Ambiguous player indices"));
        }
    }
    Ok(links)
}
fn teams(bytes: &[u8]) -> Result<Vec<u64>> {
    // Team names may be inline strings, so read only the integer identity column.
    let db = tdb_core::TdbFile::parse(bytes).map_err(|e| invalid(&e.to_string()))?;
    let table = db
        .table_canonical("kOtt")
        .ok_or_else(|| invalid("Missing teams"))?;
    let field = table
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "vfEq")
        .ok_or_else(|| invalid("Missing team index"))?;
    let mut ids = (0..u32::from(table.current_records))
        .map(|r| {
            db.read_u64(table, r, field)
                .ok_or_else(|| invalid("Unreadable team index"))
        })
        .collect::<Result<Vec<_>>>()?;
    ids.sort_unstable();
    if ids.windows(2).any(|w| w[0] == w[1]) {
        return Err(invalid("Ambiguous team indices"));
    }
    Ok(ids)
}

pub fn between(before: &[u8], after: &[u8]) -> Result<Option<RosterTableOp>> {
    // Other database families use different roster tables. Leave their existing
    // package comparison path alone, and avoid decoding unchanged-size rosters.
    let count = |bytes: &[u8]| -> Result<Option<u16>> {
        let db = tdb_core::TdbFile::parse(bytes).map_err(|e| invalid(&e.to_string()))?;
        Ok(db.table_canonical("eGlu").map(|t| t.current_records))
    };
    match (count(before)?, count(after)?) {
        (Some(b), Some(a)) if a > b => {}
        _ => return Ok(None),
    }
    let b = rows(before)?;
    let a = rows(after)?;
    if a.len() <= b.len() {
        return Ok(None);
    }
    let op = RosterTableOp {
        before: b,
        after: a,
        player_links: links(before)?,
        team_ids: teams(before)?,
    };
    if links(after)? != op.player_links || teams(after)? != op.team_ids {
        return Err(invalid("Roster growth with new or remapped player/team identities is not supported; existing-player roster additions are supported"));
    }
    validate_shape(&op)?;
    Ok(Some(op))
}

pub fn validate_shape(op: &RosterTableOp) -> Result<()> {
    if op.before.is_empty()
        || op.after.len() <= op.before.len()
        || op.after.len() > u16::MAX as usize
        || op.player_links.len() > u16::MAX as usize
        || op.team_ids.len() > u16::MAX as usize
    {
        return Err(invalid("Invalid roster addition counts"));
    }
    let fields: BTreeSet<_> = op.before[0].keys().collect();
    if fields.len() > 255
        || !fields.iter().all(|f| f.len() == 4)
        || !fields.iter().any(|f| f.as_str() == "XSWT")
        || !fields.iter().any(|f| f.as_str() == "dXSB")
    {
        return Err(invalid("Invalid roster columns"));
    }
    if op.team_ids.windows(2).any(|w| w[0] >= w[1]) {
        return Err(invalid("Duplicate or unordered team identities"));
    }
    let memberships = |rows: &[Row]| -> Result<BTreeMap<(u64, u64), usize>> {
        let mut counts = BTreeMap::new();
        for row in rows {
            if row.keys().collect::<BTreeSet<_>>() != fields {
                return Err(invalid("Inconsistent roster columns"));
            }
            // Empty stock slots can have a sentinel player index. They are preserved,
            // but only existing player/team identities may acquire a new membership.
            *counts.entry((row["dXSB"], row["XSWT"])).or_insert(0) += 1;
        }
        Ok(counts)
    };
    let before = memberships(&op.before)?;
    let after = memberships(&op.after)?;
    for ((team, player), count) in after {
        let previous = before.get(&(team, player)).copied().unwrap_or(0);
        if count > previous
            && (!op.team_ids.contains(&team) || !op.player_links.contains_key(&player) || count > 1)
        {
            return Err(invalid(
                "Roster addition has a missing player/team or duplicate membership",
            ));
        }
    }
    Ok(())
}

/// Check both membership content and the joins before applying or declaring already installed.
pub fn check_destination(bytes: &[u8], op: &RosterTableOp) -> Result<bool> {
    validate_shape(op)?;
    if links(bytes)? != op.player_links || teams(bytes)? != op.team_ids {
        return Err(invalid(
            "Player/team identities differ from this roster package; nothing applied",
        ));
    }
    let current = rows(bytes)?;
    if current == op.after {
        return Ok(true);
    }
    if current != op.before {
        return Err(invalid("The destination roster differs from this mod's baseline. Restore the original installation first; roster additions cannot overwrite a different roster"));
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EditSession, Platform, Project, ProjectRole};
    use std::path::Path;

    fn sample() -> (EditSession, RosterTableOp) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut project = Project::load(
            &root.join("anchors/ps3/nhl08/databases/nhlng.db"),
            Platform::Ps3,
            ProjectRole::Nhlng,
            &root.join("anchors/ps3/nhl08/databases/nhlng-meta.xml"),
            root.join("defs").to_str().unwrap(),
        )
        .unwrap();
        project.set_game_id(Some("nhl08".into()));
        let mut session = EditSession::begin(project);
        let before = session.bytes().to_vec();
        let old = rows(&before).unwrap();
        let (index, player) = links(&before)
            .unwrap()
            .into_iter()
            .find(|(index, player)| {
                *player == 413 && !old.iter().any(|r| r["dXSB"] == 94 && r["XSWT"] == *index)
            })
            .unwrap();
        session.add_membership(player, 94).unwrap();
        let op = between(&before, session.bytes())
            .unwrap()
            .expect("membership grows roster");
        assert!(op
            .after
            .iter()
            .any(|r| r["dXSB"] == 94 && r["XSWT"] == index));
        assert!(session.undo());
        (session, op)
    }

    #[test]
    fn replay_preserves_all_fields_and_is_idempotent() {
        let (mut session, op) = sample();
        session.set_package_roster(&op).unwrap();
        assert_eq!(rows(session.bytes()).unwrap(), op.after);
        let once = session.bytes().to_vec();
        session.set_package_roster(&op).unwrap();
        assert_eq!(session.bytes(), once);
    }

    #[test]
    fn different_roster_or_identity_is_refused_without_writing() {
        let (mut session, mut op) = sample();
        let before = session.bytes().to_vec();
        op.before[0].insert("sVRt".into(), 999);
        assert!(session.set_package_roster(&op).is_err());
        assert_eq!(session.bytes(), before);
        let (_, mut op) = sample();
        *op.player_links.values_mut().next().unwrap() += 1;
        assert!(session.set_package_roster(&op).is_err());
        assert_eq!(session.bytes(), before);
    }

    #[test]
    fn duplicate_membership_unknown_player_and_missing_columns_are_rejected() {
        let (_, op) = sample();
        let mut bad = op.clone();
        bad.after.push(op.after[0].clone());
        assert!(validate_shape(&bad).is_err());
        let mut bad = op.clone();
        bad.after
            .last_mut()
            .unwrap()
            .insert("XSWT".into(), u64::MAX);
        assert!(validate_shape(&bad).is_err());
        let mut bad = op;
        bad.after[0].remove("XSWT");
        assert!(validate_shape(&bad).is_err());
    }
}
