//! The Layer 2 **entity model** — typed `Player` / `RosterSlot` / `Team` assembled over the verified
//! join model, so edits operate on entities rather than bare rows (EDIT-MODEL Layer 2).
//!
//! A [`RosterModel::build`] resolves the whole cluster graph from a decoded [`TdbFile`] **by
//! construction**:
//!
//! * **Player** — a `ubPc` bio (key `wBIz`) plus the companion rows that share `ubPc.wBIz` 1:1:
//!   `dSvy` (skater AI) + `xmja` (skater equipment) for skaters, `mHuy` (goalie AI) + `fMVl` (goalie
//!   equipment) for goalies. Skater/goalie is decided by the `ubPc.vjla` position code (`4` = goalie).
//!   *Verified against the PS3 NHL Legacy anchor:* 5066 skaters ↔ 5066 `dSvy`/`xmja`, 566 goalies ↔
//!   566 `mHuy`/`fMVl`, all four companions keyed on `wBIz`.
//! * **RosterSlot** — an `eGlu` row (key `toWX`) bridged to its player via
//!   `eGlu.XSWT → ZBac.vfEq` then `ZBac.ykFq → ubPc.wBIz`, and to its team via `eGlu.dXSB → kOtt.vfEq`.
//!   Carries the per-slot jersey (`sVRt`), captaincy (`mCcl`), and roster status (`ZBac.RREB`).
//! * **Team** — a `kOtt` row (key `vfEq`).
//!
//! All ids below are the **studio** convention; [`TdbFile::table_canonical`] / [`RawId::canonical`]
//! resolve them across the big-endian on-disk reversal.

use std::collections::HashMap;
use tdb_core::{FieldDescriptor, Table, TdbFile};

/// Studio ids for the player-cluster tables and fields (verified present on the anchors).
pub mod ids {
    // Tables.
    pub const BIO: &str = "ubPc";
    pub const INDEX: &str = "ZBac";
    pub const SKATER_AI: &str = "dSvy";
    pub const SKATER_EQUIP: &str = "xmja";
    pub const GOALIE_AI: &str = "mHuy";
    pub const GOALIE_EQUIP: &str = "fMVl";
    pub const ROSTER_LINES: &str = "eGlu";
    pub const TEAM: &str = "kOtt";

    // NHL07-only team-name join. In NHL 09→21 `kOtt` (`exhibitionteams`) carries the name fields
    // inline; in NHL07 it is a thin index whose `TEAM_NAME_TABLE_DISC` (`table`) selects one of the
    // name-bearing tables below and `TEAM_NAME_ID` (`teamid`) is the row's key within it. See the
    // studio-domain team-name resolver.
    pub const STOCK_TEAM: &str = "HqzU"; // stockteamtable
    pub const CUSTOM_TEAM: &str = "fLEY"; // customteamtable
    pub const TEAM_NAME_ID: &str = "mCXg"; // exhibitionteams.teamid → name table key
    pub const TEAM_NAME_TABLE_DISC: &str = "RREB"; // exhibitionteams.table (0=stock, 1=custom)

    // Fields.
    pub const PLAYER_ID: &str = "wBIz"; // ubPc key + companion join
    pub const POSITION: &str = "vjla"; // ubPc position code (0..4, 4 = goalie)
    pub const INDEX_KEY: &str = "vfEq"; // ZBac key (XSWT target)
    pub const INDEX_TO_BIO: &str = "ykFq"; // ZBac → ubPc.wBIz
    // NOTE (Spike S4): `ZBac.RREB` is the PLAYER-TYPE discriminator (TNHLPlayerType), not roster
    // status, so the model no longer reads it. The real dressed/scratched signal is the per-slot
    // `eGlu.hSZj` boolean below.
    pub const SLOT_KEY: &str = "toWX"; // eGlu key
    pub const SLOT_TO_INDEX: &str = "XSWT"; // eGlu → ZBac.vfEq
    pub const SLOT_TEAM: &str = "dXSB"; // eGlu → kOtt.vfEq
    pub const SLOT_JERSEY: &str = "sVRt"; // eGlu jersey number (7-bit)
    pub const SLOT_CAPTAINCY: &str = "mCcl"; // eGlu captaincy (2-bit)
    pub const SLOT_DRESSED: &str = "hSZj"; // eGlu "rosterstatus" dressed flag (1-bit: 1=dressed, 0=scratched)
    pub const TEAM_KEY: &str = "vfEq"; // kOtt key
}

/// Player position from `ubPc.vjla` (operator-confirmed code space).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Center,
    LeftWing,
    RightWing,
    Defense,
    Goalie,
    /// A code outside the confirmed `0..=4` space (kept rather than guessed).
    Unknown(i64),
}

impl Position {
    pub fn from_raw(v: i64) -> Self {
        match v {
            0 => Position::Center,
            1 => Position::LeftWing,
            2 => Position::RightWing,
            3 => Position::Defense,
            4 => Position::Goalie,
            other => Position::Unknown(other),
        }
    }

    pub fn is_goalie(self) -> bool {
        matches!(self, Position::Goalie)
    }
}

/// Whether a player is assembled as a skater or a goalie (decides the valid companion pair).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerKind {
    Skater,
    Goalie,
}

/// A player: the bio row plus its resolved companion rows. Each `*_record` is the row index of the
/// companion that shares this player's `wBIz`, or `None` if absent.
#[derive(Clone, Debug)]
pub struct Player {
    pub bio_record: u32,
    pub player_id: u64, // wBIz
    pub position: Position,
    pub kind: PlayerKind,
    pub skater_ai: Option<u32>,    // dSvy
    pub skater_equip: Option<u32>, // xmja
    pub goalie_ai: Option<u32>,    // mHuy
    pub goalie_equip: Option<u32>, // fMVl
}

impl Player {
    /// True iff *both* companions this player's `kind` uses in the modern (fat) schema are present —
    /// AI + equipment. This is the unconditional cluster check; whole-DB integrity uses the
    /// schema-aware [`RosterModel::companion_violations`] instead, which drops the equipment
    /// requirement on gen6 titles (NHL07) that ship no equipment table.
    pub fn companions_complete(&self) -> bool {
        match self.kind {
            PlayerKind::Skater => self.skater_ai.is_some() && self.skater_equip.is_some(),
            PlayerKind::Goalie => self.goalie_ai.is_some() && self.goalie_equip.is_some(),
        }
    }
}

/// A team: the `kOtt` row.
#[derive(Clone, Debug)]
pub struct Team {
    pub record: u32,
    pub team_id: u64, // vfEq
}

/// A roster slot: an `eGlu` row resolved to its team and (through the two-hop index bridge) its
/// player.
#[derive(Clone, Debug)]
pub struct RosterSlot {
    pub record: u32,
    pub slot_key: u64, // toWX
    pub team_id: u64,  // dXSB → kOtt.vfEq
    /// `eGlu.XSWT` (the `ZBac.vfEq` this slot points at), if populated.
    pub index_key: Option<u64>,
    /// The player this slot resolves to (XSWT → ZBac.vfEq → ZBac.ykFq → ubPc.wBIz), if it resolves.
    pub player_id: Option<u64>,
    /// Lineup status from the per-slot `eGlu.hSZj` boolean: `Some(true)` = dressed, `Some(false)` =
    /// scratched (out of the lineup), `None` if the field is absent. ("In minors" is *not* stored
    /// here — it is a front-end depth-chart identifier derived from the player's main team.)
    pub dressed: Option<bool>,
    pub jersey: Option<u64>,    // sVRt
    pub captaincy: Option<u64>, // mCcl
}

/// Errors assembling the entity model.
#[derive(Debug)]
pub enum EntityError {
    /// A structurally-required table is absent from this db.
    MissingTable(&'static str),
    /// A structurally-required field is absent from a present table.
    MissingField(&'static str, &'static str),
}

impl std::fmt::Display for EntityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityError::MissingTable(t) => write!(f, "required table {t} absent"),
            EntityError::MissingField(t, fld) => write!(f, "required field {t}.{fld} absent"),
        }
    }
}
impl std::error::Error for EntityError {}

/// One resolved player-index entry (`ZBac` row): the bio it points at.
#[derive(Clone, Copy, Debug)]
struct IndexEntry {
    player_id: u64, // ykFq
}

/// The assembled roster graph for one database.
#[derive(Debug, Default)]
pub struct RosterModel {
    pub teams: Vec<Team>,
    pub players: Vec<Player>,
    pub slots: Vec<RosterSlot>,
    team_by_id: HashMap<u64, usize>,
    player_by_id: HashMap<u64, usize>,
    /// `wBIz`s that appear in the `ZBac` player index — the players the game actually loads. A
    /// `ubPc` bio row with *no* `ZBac` entry (and no roster slot) is an orphan stub the game never
    /// references; it must not be treated as a live player (e.g. by integrity predicates). Verified
    /// on the NHL 09 anchor: 4 vanilla `ubPc` rows carry no `ZBac` entry, no companions, no slot.
    referenced: std::collections::HashSet<u64>,
    /// Whether the skater/goalie *equipment* companion tables exist in this db's schema. Gen6 titles
    /// (NHL07) split ratings onto the AI tables (`dSvy`/`mHuy`) but ship no separate equipment tables
    /// (`xmja`/`fMVl`), so their absence is a schema shape, not a broken cluster — the companion
    /// invariant requires the equip row only when its table is present. Probed from table presence,
    /// never from an empty companion map (which is ambiguous: absent vs. present-but-unpopulated).
    has_skater_equip: bool,
    has_goalie_equip: bool,
}

/// Resolve a field on `table` by its studio id (handling the BE reversal).
fn field<'a>(db: &TdbFile, table: &'a Table, studio: &str) -> Option<&'a FieldDescriptor> {
    table
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == studio)
}

/// Required field or `EntityError::MissingField`.
fn req_field<'a>(
    db: &TdbFile,
    table: &'a Table,
    tname: &'static str,
    fname: &'static str,
) -> Result<&'a FieldDescriptor, EntityError> {
    field(db, table, fname).ok_or(EntityError::MissingField(tname, fname))
}

/// Build a `wBIz → record` map for a companion table (skips it gracefully if absent).
fn companion_map(db: &TdbFile, table_id: &str) -> HashMap<u64, u32> {
    let mut map = HashMap::new();
    if let Some(t) = db.table_canonical(table_id) {
        if let Some(f) = field(db, t, ids::PLAYER_ID) {
            for i in 0..t.current_records as u32 {
                if let Some(v) = db.read_u64(t, i, f) {
                    map.insert(v, i);
                }
            }
        }
    }
    map
}

impl RosterModel {
    /// Assemble the entity model from a decoded TDB database.
    pub fn build(db: &TdbFile) -> Result<Self, EntityError> {
        let mut model = RosterModel::default();

        // --- Teams (kOtt) ---
        let team_t = db
            .table_canonical(ids::TEAM)
            .ok_or(EntityError::MissingTable(ids::TEAM))?;
        let team_key = req_field(db, team_t, ids::TEAM, ids::TEAM_KEY)?;
        for i in 0..team_t.current_records as u32 {
            if let Some(team_id) = db.read_u64(team_t, i, team_key) {
                model.team_by_id.insert(team_id, model.teams.len());
                model.teams.push(Team {
                    record: i,
                    team_id,
                });
            }
        }

        // --- Players (ubPc + companions) ---
        let bio_t = db
            .table_canonical(ids::BIO)
            .ok_or(EntityError::MissingTable(ids::BIO))?;
        let bio_key = req_field(db, bio_t, ids::BIO, ids::PLAYER_ID)?;
        let bio_pos = req_field(db, bio_t, ids::BIO, ids::POSITION)?;

        let skater_ai = companion_map(db, ids::SKATER_AI);
        let skater_equip = companion_map(db, ids::SKATER_EQUIP);
        let goalie_ai = companion_map(db, ids::GOALIE_AI);
        let goalie_equip = companion_map(db, ids::GOALIE_EQUIP);
        // Table presence (not map emptiness): a gen6 db lacks these tables entirely.
        model.has_skater_equip = db.table_canonical(ids::SKATER_EQUIP).is_some();
        model.has_goalie_equip = db.table_canonical(ids::GOALIE_EQUIP).is_some();

        for i in 0..bio_t.current_records as u32 {
            let Some(player_id) = db.read_u64(bio_t, i, bio_key) else {
                continue;
            };
            let position = Position::from_raw(db.read_u64(bio_t, i, bio_pos).unwrap_or(0) as i64);
            let kind = if position.is_goalie() {
                PlayerKind::Goalie
            } else {
                PlayerKind::Skater
            };
            model.player_by_id.insert(player_id, model.players.len());
            model.players.push(Player {
                bio_record: i,
                player_id,
                position,
                kind,
                skater_ai: skater_ai.get(&player_id).copied(),
                skater_equip: skater_equip.get(&player_id).copied(),
                goalie_ai: goalie_ai.get(&player_id).copied(),
                goalie_equip: goalie_equip.get(&player_id).copied(),
            });
        }

        // --- Player index (ZBac): vfEq → {ykFq, RREB} ---
        let index_t = db
            .table_canonical(ids::INDEX)
            .ok_or(EntityError::MissingTable(ids::INDEX))?;
        let idx_key = req_field(db, index_t, ids::INDEX, ids::INDEX_KEY)?;
        let idx_bio = req_field(db, index_t, ids::INDEX, ids::INDEX_TO_BIO)?;
        let mut index: HashMap<u64, IndexEntry> = HashMap::new();
        for i in 0..index_t.current_records as u32 {
            let (Some(vfeq), Some(player_id)) = (
                db.read_u64(index_t, i, idx_key),
                db.read_u64(index_t, i, idx_bio),
            ) else {
                continue;
            };
            index.insert(vfeq, IndexEntry { player_id });
            model.referenced.insert(player_id);
        }

        // --- Roster slots (eGlu) ---
        let slot_t = db
            .table_canonical(ids::ROSTER_LINES)
            .ok_or(EntityError::MissingTable(ids::ROSTER_LINES))?;
        // The eGlu key (`toWX`) is the row's own identity, minted by structural edits. Pre-nhl08
        // schemas (NHL 07) don't carry it — its absence only affects structural roster editing, not
        // reading, so we make it optional here and fall back to the record index as the slot identity.
        // Reads never consume `slot_key`; the edit path looks the field up independently and errors
        // cleanly if it's missing.
        let slot_key = field(db, slot_t, ids::SLOT_KEY);
        let slot_team = req_field(db, slot_t, ids::ROSTER_LINES, ids::SLOT_TEAM)?;
        let slot_index = req_field(db, slot_t, ids::ROSTER_LINES, ids::SLOT_TO_INDEX)?;
        let slot_jersey = field(db, slot_t, ids::SLOT_JERSEY);
        let slot_captaincy = field(db, slot_t, ids::SLOT_CAPTAINCY);
        let slot_dressed = field(db, slot_t, ids::SLOT_DRESSED);
        let slot_sentinel = if slot_index.bit_width >= 64 {
            u64::MAX
        } else {
            (1u64 << slot_index.bit_width) - 1
        };

        for i in 0..slot_t.current_records as u32 {
            // With a key field, skip rows whose key can't be read (malformed); without one
            // (older schemas), the record index is a stable stand-in identity for reads.
            let key = match slot_key {
                Some(f) => match db.read_u64(slot_t, i, f) {
                    Some(k) => k,
                    None => continue,
                },
                None => i as u64,
            };
            let team_id = db.read_u64(slot_t, i, slot_team).unwrap_or(0);
            let raw_index = db.read_u64(slot_t, i, slot_index);
            let index_key = match raw_index {
                Some(v) if v != 0 && v != slot_sentinel => Some(v),
                _ => None,
            };
            let entry = index_key.and_then(|k| index.get(&k));
            if let Some(e) = entry {
                model.referenced.insert(e.player_id);
            }
            model.slots.push(RosterSlot {
                record: i,
                slot_key: key,
                team_id,
                index_key,
                player_id: entry.map(|e| e.player_id),
                dressed: slot_dressed.and_then(|f| db.read_u64(slot_t, i, f)).map(|v| v != 0),
                jersey: slot_jersey.and_then(|f| db.read_u64(slot_t, i, f)),
                captaincy: slot_captaincy.and_then(|f| db.read_u64(slot_t, i, f)),
            });
        }

        Ok(model)
    }

    /// A player by its `wBIz` id.
    pub fn player(&self, player_id: u64) -> Option<&Player> {
        self.player_by_id.get(&player_id).map(|&i| &self.players[i])
    }

    /// A team by its `vfEq` id.
    pub fn team(&self, team_id: u64) -> Option<&Team> {
        self.team_by_id.get(&team_id).map(|&i| &self.teams[i])
    }

    /// The player a slot resolves to (through the verified two-hop bridge).
    pub fn slot_player(&self, slot: &RosterSlot) -> Option<&Player> {
        slot.player_id.and_then(|id| self.player(id))
    }

    /// The team a slot belongs to.
    pub fn slot_team(&self, slot: &RosterSlot) -> Option<&Team> {
        self.team(slot.team_id)
    }

    /// Every slot owned by a team.
    pub fn team_roster(&self, team_id: u64) -> Vec<&RosterSlot> {
        self.slots.iter().filter(|s| s.team_id == team_id).collect()
    }

    /// Whether this `wBIz` is referenced by the game — it has a `ZBac` index entry or fills a
    /// roster slot. Orphan `ubPc` bio stubs (no index, no slot) return `false`; the game never
    /// loads them, so integrity checks should not treat them as live players.
    pub fn is_referenced(&self, player_id: u64) -> bool {
        self.referenced.contains(&player_id)
    }

    /// Players whose companion pair does not match their kind — the Layer 2 cluster invariant
    /// violated. Empty on clean data; the shared predicate Layer 4 will reuse this.
    ///
    /// Unreferenced orphan bio stubs are excluded: a `ubPc` row the game never loads (no `ZBac`
    /// entry, no roster slot) missing its companions is dead vanilla data, not a structural fault
    /// that can affect the game. Verified on the NHL 09 anchor (4 such rows ship in the retail db).
    ///
    /// The *equipment* companion (`xmja`/`fMVl`) is required only when its table exists in this db:
    /// gen6 schemas (NHL07) carry no equipment tables at all, so a skater/goalie is not "incomplete"
    /// for lacking a row in a table its title never had. The AI companion (`dSvy`/`mHuy`) is always
    /// required — those tables exist across the supported range.
    pub fn companion_violations(&self) -> Vec<&Player> {
        self.players
            .iter()
            .filter(|p| self.is_referenced(p.player_id) && !self.player_cluster_complete(p))
            .collect()
    }

    /// Whether a player's required companions are present, accounting for which companion tables this
    /// db's schema actually carries (see [`RosterModel::companion_violations`]).
    fn player_cluster_complete(&self, p: &Player) -> bool {
        match p.kind {
            PlayerKind::Skater => {
                p.skater_ai.is_some() && (!self.has_skater_equip || p.skater_equip.is_some())
            }
            PlayerKind::Goalie => {
                p.goalie_ai.is_some() && (!self.has_goalie_equip || p.goalie_equip.is_some())
            }
        }
    }

    pub fn skater_count(&self) -> usize {
        self.players
            .iter()
            .filter(|p| p.kind == PlayerKind::Skater)
            .count()
    }

    pub fn goalie_count(&self) -> usize {
        self.players
            .iter()
            .filter(|p| p.kind == PlayerKind::Goalie)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_codes() {
        assert_eq!(Position::from_raw(0), Position::Center);
        assert_eq!(Position::from_raw(4), Position::Goalie);
        assert!(Position::from_raw(4).is_goalie());
        assert_eq!(Position::from_raw(9), Position::Unknown(9));
    }

    #[test]
    fn companion_completeness_rule() {
        let skater = Player {
            bio_record: 0,
            player_id: 1,
            position: Position::Center,
            kind: PlayerKind::Skater,
            skater_ai: Some(0),
            skater_equip: Some(0),
            goalie_ai: None,
            goalie_equip: None,
        };
        assert!(skater.companions_complete());

        let broken_goalie = Player {
            kind: PlayerKind::Goalie,
            goalie_ai: Some(0),
            goalie_equip: None, // missing → incomplete
            ..skater.clone()
        };
        assert!(!broken_goalie.companions_complete());
    }
}
