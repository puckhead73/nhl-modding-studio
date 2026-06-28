//! Read-side **view-models** — the presentation layer that turns raw cells into display-ready
//! fields by joining a decoded [`TdbFile`] with the [`FieldMetaIndex`] (labels, ranges, updatable
//! flag) and the [`EnumRegistry`] (value→label). This is the backbone the entity views (Player,
//! Team, Arena, …) build on, and the data the UI's progressive-disclosure "all fields" panel shows.
//!
//! ## Id convention (critical)
//!
//! When we hold a [`FieldDescriptor`] we resolve metadata and enums without ever reversing ids by
//! hand:
//! * **metadata** is keyed by the **on-disk** id, which is exactly [`RawId::raw`] — endian-correct on
//!   both big- and little-endian files (no `reverse_id` needed, which would be wrong on PS4);
//! * **enums/relations** speak the **studio** id, which is [`RawId::canonical`].

use std::collections::HashMap;

use tdb_core::{FieldDescriptor, Table, TdbFile};
use tdb_defs::{FieldMetaIndex, FieldType};
use tdb_enums::EnumRegistry;
use tdb_relations::entity::ids;
use tdb_relations::{PlayerKind, RosterModel};

/// The value class of a field (mirrors [`tdb_defs::FieldType`] without its `Other(String)` payload).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Integer,
    String,
    Real,
    Other,
}

impl FieldKind {
    fn from_meta(kind: &FieldType) -> Self {
        match kind {
            FieldType::Integer => FieldKind::Integer,
            FieldType::String => FieldKind::String,
            FieldType::Real => FieldKind::Real,
            FieldType::Other(_) => FieldKind::Other,
        }
    }

    /// A stable lowercase identifier for DTOs.
    pub fn as_str(self) -> &'static str {
        match self {
            FieldKind::Integer => "integer",
            FieldKind::String => "string",
            FieldKind::Real => "real",
            FieldKind::Other => "other",
        }
    }
}

/// The raw value as read from the record (before sign/enum interpretation).
#[derive(Clone, Debug)]
pub enum FieldRaw {
    /// Raw stored bits of a fixed-bit field.
    Int(u64),
    /// A NUL-trimmed inline string (lossy UTF-8).
    Text(String),
}

/// One field of one record, ready to display or edit.
#[derive(Clone, Debug)]
pub struct FieldView {
    /// Studio (community) table id, e.g. `ubPc`.
    pub studio_table: String,
    /// Studio (community) field id, e.g. `vjla`.
    pub studio_field: String,
    /// Human field name from the metadata (falls back to the studio field id when absent).
    pub name: String,
    pub kind: FieldKind,
    pub raw: FieldRaw,
    /// Sign/sentinel-decoded value for integer fields; `None` for strings.
    pub decoded: Option<i64>,
    /// What to show: the enum label, the decoded number, or the string text.
    pub display: String,
    /// The enum label for this value, if the field is enum-bound and the value is a member.
    pub enum_label: Option<String>,
    pub range_low: Option<i64>,
    pub range_high: Option<i64>,
    /// `true` iff the metadata marks the field updatable (the edit-entry guardrail).
    pub editable: bool,
    pub is_string: bool,
}

/// Build a [`FieldView`] for one field of one record. Never panics: a field with no metadata still
/// yields a usable (non-editable) view so the layer is robust across title/platform variants.
pub fn read_field(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    table: &Table,
    index: u32,
    field: &FieldDescriptor,
) -> FieldView {
    let studio_table = table.id.canonical(db.endian);
    let studio_field = field.id.canonical(db.endian);
    // Metadata is keyed by the on-disk id — `raw()` is exactly that, on both byte orders.
    let fm = meta.field(&table.id.raw(), &field.id.raw());

    let name = fm
        .map(|m| m.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| studio_field.clone());
    let kind = fm
        .map(|m| FieldKind::from_meta(&m.kind))
        .unwrap_or(FieldKind::Other);
    let editable = fm.map(|m| m.updatable).unwrap_or(false);
    let range_low = fm.and_then(|m| m.range_low);
    let range_high = fm.and_then(|m| m.range_high);
    let is_string = kind == FieldKind::String;

    if is_string {
        let text = db
            .read_inline_str(table, index, field)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        FieldView {
            studio_table,
            studio_field,
            name,
            kind,
            raw: FieldRaw::Text(text.clone()),
            decoded: None,
            display: text,
            enum_label: None,
            range_low,
            range_high,
            editable,
            is_string,
        }
    } else {
        let raw_bits = db.read_u64(table, index, field).unwrap_or(0);
        let decoded = fm.map(|m| m.decode_raw(raw_bits)).unwrap_or(raw_bits as i64);
        let enum_label = enums
            .label(&studio_table, &studio_field, decoded)
            .map(|s| s.to_string());
        let display = enum_label
            .clone()
            .unwrap_or_else(|| decoded.to_string());
        FieldView {
            studio_table,
            studio_field,
            name,
            kind,
            raw: FieldRaw::Int(raw_bits),
            decoded: Some(decoded),
            display,
            enum_label,
            range_low,
            range_high,
            editable,
            is_string,
        }
    }
}

/// Every field of one record of a studio-named table, as [`FieldView`]s (the "all fields" panel).
/// Returns an empty vec if the table is absent from this db variant.
pub fn entity_fields(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    studio_table: &str,
    index: u32,
) -> Vec<FieldView> {
    let Some(table) = db.table_canonical(studio_table) else {
        return Vec::new();
    };
    table
        .fields
        .iter()
        .map(|f| read_field(db, meta, enums, table, index, f))
        .collect()
}

// ---------------------------------------------------------------------------
// Player view-models
// ---------------------------------------------------------------------------

/// Curated bio fields (by metadata `name`) for the Player detail header, in display order.
const PLAYER_IDENTITY: &[&str] = &[
    "firstname",
    "lastname",
    "position",
    "height",
    "weight",
    "handedness",
    "intlcountry",
    "jersey",
    "hasportrait",
    "headid",
];

/// A player's roster membership on one team (from the entity model's resolved slots).
#[derive(Clone, Debug)]
pub struct RosterMembership {
    pub team_id: u64,
    pub jersey: Option<u64>,
    pub captaincy: Option<u64>,
    pub roster_status: Option<u64>,
}

/// A compact player row for the master list.
#[derive(Clone, Debug)]
pub struct PlayerSummary {
    pub player_id: u64,
    pub first_name: String,
    pub last_name: String,
    pub position_label: String,
    pub team_id: Option<u64>,
}

/// The full Player detail: curated identity, ratings (the kind-correct companion's fields), roster
/// memberships, and the complete bio field list for progressive disclosure.
#[derive(Clone, Debug)]
pub struct PlayerView {
    pub player_id: u64,
    pub bio_record: u32,
    pub kind: &'static str,
    pub identity: Vec<FieldView>,
    pub ratings: Vec<FieldView>,
    /// The companion record index the `ratings` fields belong to (`dSvy`/`mHuy`) — needed to edit
    /// them. `None` if the player has no companion (a broken cluster).
    pub ratings_record: Option<u32>,
    /// The companion table id the `ratings` fields belong to (`dSvy` or `mHuy`).
    pub ratings_table: &'static str,
    pub memberships: Vec<RosterMembership>,
    pub all_fields: Vec<FieldView>,
}

/// Find the field descriptor of `studio_table` whose metadata `name` matches (handles the id
/// reversal via on-disk `raw()` lookups). Used to read a named field without hardcoding its id.
fn descriptor_by_name<'a>(
    meta: &FieldMetaIndex,
    table: &'a Table,
    name: &str,
) -> Option<&'a FieldDescriptor> {
    table.fields.iter().find(|f| {
        meta.field(&table.id.raw(), &f.id.raw())
            .map(|m| m.name == name)
            .unwrap_or(false)
    })
}

/// Build the master list of players from the assembled entity model.
pub fn player_summaries(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    model: &RosterModel,
) -> Vec<PlayerSummary> {
    let Some(bio) = db.table_canonical(ids::BIO) else {
        return Vec::new();
    };
    // Resolve the three summary descriptors once, then read each player's bio record.
    let first_f = descriptor_by_name(meta, bio, "firstname");
    let last_f = descriptor_by_name(meta, bio, "lastname");
    let pos_f = descriptor_by_name(meta, bio, "position");

    // First team per player (for the list's team column).
    let mut team_of: HashMap<u64, u64> = HashMap::new();
    for slot in &model.slots {
        if let Some(pid) = slot.player_id {
            team_of.entry(pid).or_insert(slot.team_id);
        }
    }

    let read_str = |f: Option<&FieldDescriptor>, rec: u32| -> String {
        f.and_then(|f| db.read_inline_str(bio, rec, f))
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default()
    };

    model
        .players
        .iter()
        .map(|p| {
            let position_label = pos_f
                .map(|f| read_field(db, meta, enums, bio, p.bio_record, f).display)
                .unwrap_or_default();
            PlayerSummary {
                player_id: p.player_id,
                first_name: read_str(first_f, p.bio_record),
                last_name: read_str(last_f, p.bio_record),
                position_label,
                team_id: team_of.get(&p.player_id).copied(),
            }
        })
        .collect()
}

/// Build the full Player detail view for one `wBIz`. Returns `None` if the player is not in the model.
pub fn player_view(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    model: &RosterModel,
    player_id: u64,
) -> Option<PlayerView> {
    let player = model.player(player_id)?;
    let all_fields = entity_fields(db, meta, enums, ids::BIO, player.bio_record);

    // Curated identity = the PLAYER_IDENTITY names, in order, picked from the bio fields.
    let identity = PLAYER_IDENTITY
        .iter()
        .filter_map(|name| all_fields.iter().find(|f| &f.name == name).cloned())
        .collect();

    // Ratings come from the kind-correct companion record the model already resolved.
    let (companion_table, companion_record) = match player.kind {
        PlayerKind::Goalie => (ids::GOALIE_AI, player.goalie_ai),
        PlayerKind::Skater => (ids::SKATER_AI, player.skater_ai),
    };
    let ratings = companion_record
        .map(|rec| entity_fields(db, meta, enums, companion_table, rec))
        .unwrap_or_default();

    let memberships = model
        .slots
        .iter()
        .filter(|s| s.player_id == Some(player_id))
        .map(|s| RosterMembership {
            team_id: s.team_id,
            jersey: s.jersey,
            captaincy: s.captaincy,
            roster_status: s.roster_status,
        })
        .collect();

    Some(PlayerView {
        player_id,
        bio_record: player.bio_record,
        kind: match player.kind {
            PlayerKind::Goalie => "goalie",
            PlayerKind::Skater => "skater",
        },
        identity,
        ratings,
        ratings_record: companion_record,
        ratings_table: companion_table,
        memberships,
        all_fields,
    })
}
