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

use crate::assets::{probe_birthdate_encoding, BirthdateEncoding};

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
    /// The full `(raw, label)` vocabulary when the field is enum-bound — powers edit-time dropdowns.
    /// Empty for non-enum fields.
    pub enum_options: Vec<(i64, String)>,
    /// EA-semantic edit offset: the front-facing value is `decoded + edit_offset`, so an editor can
    /// let the user type the friendly number (2011, 209 lb) and stage `input - edit_offset`. `None`
    /// for fields edited as-is.
    pub edit_offset: Option<i64>,
    /// When true the friendly value (inches = `decoded + edit_offset`) is presented/parsed as `ft'in"`.
    pub edit_height: bool,
    /// Fixed-point scale: the front-facing value is `decoded * edit_scale` (SV% ×0.001, GAA ×0.01),
    /// so an editor accepts the decimal and stages `round(input / edit_scale)`. `None` = plain.
    pub edit_scale: Option<f64>,
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
    // A field is editable as a scalar iff its metadata marks it updatable **and** it is not a
    // key/identity field. Some schemas (e.g. NHL07 `vfEq`) ship a primary key with `update=True`;
    // editing a key in place would orphan its foreign keys, so the edit-entry guardrail refuses it
    // (`EditSession::set_field` enforces the same rule on the write side). Keys are still shown as
    // read-only cells for disclosure.
    let editable = fm.map(|m| m.updatable && !m.key).unwrap_or(false);
    let range_low = fm.and_then(|m| m.range_low);
    let range_high = fm.and_then(|m| m.range_high);
    let is_string = kind == FieldKind::String;
    let is_gen6 = crate::attributes::is_gen6_style_schema(db);
    // Full vocabulary for enum-bound fields (empty otherwise) — powers edit-time dropdowns.
    let mut enum_options: Vec<(i64, String)> = enums
        .for_field(&studio_table, &studio_field)
        .map(|d| d.entries.iter().map(|e| (e.raw, e.label.clone())).collect())
        .unwrap_or_default();
    // Synthesized vocab for fields with no TOML enum (Potential Range/Accuracy, gen6 player styles).
    if enum_options.is_empty() {
        if let Some(opts) = crate::attributes::synthetic_enum(&studio_table, &studio_field, is_gen6) {
            enum_options = opts;
        }
    }
    // NHL07 gen6 uses a different country / birth-state code table than the modern TOML — override
    // nationality (whose European codes the modern vocab mislabels) and fill the birth-state gaps.
    crate::attributes::apply_gen6_geo_vocab(&studio_table, &studio_field, is_gen6, &mut enum_options);
    // Era-correct rating scale, classified from the field's own metadata (name prefix + declared
    // range): 6-bit 0..63 storage shows/edits +36 (the 36..99 scale); PS4's 7-bit 0..99 stores the
    // human value directly (offset 0). See `attributes::rating_offset`.
    // Era-correct rating scale. The 6-bit case's offset is data-probed: NHL 10→Legacy show +36
    // (36..99), NHL07 gen6 shows +50 (50..99). `rating_offset` returns the +36 sentinel for any
    // 6-bit rating; swap in the probed value (the scan runs only for a real 6-bit rating field).
    let rating = crate::attributes::rating_offset(&studio_table, &name, range_low, range_high)
        .map(|off| if off == 36 { sixbit_rating_offset(db) } else { off });
    // `table` IS `ubPc` whenever the birthdate fields are being read, so the probe needs no
    // extra plumbing. Lazy: it only runs for `JKLp`/`Kswi`, and returns on the first record
    // that settles the convention.
    let birthdate = || probe_birthdate_encoding(db, table);
    // `table` IS `ubPc` when the birth-year field is read, so the base probe needs no extra plumbing;
    // it only actually runs for the `ubPc.qFnd` arm below.
    let year_base = || birth_year_base(db, table);
    // Draft-year base is schema-probed (modern `draftyear` = 1900; NHL07 gen6 `yeardrafted` = 1959).
    let draft_base = || draft_year_base(db, meta);
    // Weight base: +120 lb on NHL 10→Legacy, +150 on NHL07 gen6 (raw 0..125 → 150..275, verified);
    // gated by the same data-probed gen6 signal as the rating scale. Only evaluated for the weight
    // field itself.
    let weight_base = || if sixbit_rating_offset(db) == 50 { 150 } else { 120 };
    let (edit_offset, edit_height) =
        ea_edit_form(&studio_table, &studio_field, birthdate, year_base, draft_base, weight_base);
    let edit_offset = edit_offset.or(rating.filter(|off| *off != 0));
    let edit_scale = ea_edit_scale(&studio_table, &studio_field);

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
            enum_options,
            edit_offset,
            edit_height,
            edit_scale,
            range_low,
            range_high,
            editable,
            is_string,
        }
    } else {
        let raw_bits = db.read_u64(table, index, field).unwrap_or(0);
        let decoded = fm.map(|m| m.decode_raw(raw_bits)).unwrap_or(raw_bits as i64);
        // Label from the resolved vocabulary (TOML + synthetic + gen6 overrides all flow through
        // `enum_options`, so the label and the dropdown never disagree).
        let enum_label = enum_options.iter().find(|(r, _)| *r == decoded).map(|(_, l)| l.clone());
        // Precedence: enum label → EA-semantic formatting (height/weight/…) → rating scale → the
        // plain number.
        let display = enum_label
            .clone()
            .or_else(|| {
                ea_semantic_display(
                    &studio_table, &studio_field, decoded, birthdate, year_base, draft_base, weight_base,
                )
            })
            .or_else(|| rating.map(|off| (decoded + off).to_string()))
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
            enum_options,
            edit_offset,
            edit_height,
            edit_scale,
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
    /// The team's resolved display name (the human-readable surface), when it resolves.
    pub team_name: Option<String>,
    pub jersey: Option<u64>,
    pub captaincy: Option<u64>,
    /// Lineup status from `eGlu.hSZj`: `Some(true)` dressed, `Some(false)` scratched (Spike S4).
    pub dressed: Option<bool>,
}

/// A compact player row for the master list and the team depth chart.
#[derive(Clone, Debug)]
pub struct PlayerSummary {
    pub player_id: u64,
    pub first_name: String,
    pub last_name: String,
    pub position_label: String,
    pub team_id: Option<u64>,
    /// Bio `jersey` (`sVRt`), raw 0..127.
    pub jersey_number: Option<i64>,
    /// Player/Goalie Style (archetype) enum label from the ratings companion's `QgFs`.
    pub style_label: Option<String>,
    /// Bio `active` (`kKYN`, 1-bit) — the real active flag (NOT `tEok`/`c_potential`).
    pub is_active: Option<bool>,
    /// Derived EA Overall: skaters via the ported regression/weighted-average formula, goalies
    /// from the stored `overall` rating — both on the display scale (36..99).
    pub overall: Option<i64>,
    /// NHL-rights team from bio `proteam` (`dbBW`, 1-indexed; 0/-1 = none), decoded to the
    /// canonical team id — drives the depth chart's "in the system" bucket.
    pub rights_team_id: Option<u64>,
    /// The bio record index — lets list-driven surfaces (jersey grid) edit `ubPc` cells directly.
    pub bio_record: u32,
    /// Cap hit in dollars (`kKhd` × $5000; raw 0 = no contract → `None`).
    pub cap_hit_dollars: Option<u64>,
    /// Contract term in years (`IhDG`; raw 0 → `None`).
    pub contract_years: Option<u32>,
    /// Derived from the cap-hit field, mirroring the Player Data tab's wording.
    pub contract_status: Option<String>,
    /// Two-way contract flag (`LoVD`).
    pub is_two_way: Option<bool>,
    /// `true` when this player is a goaltender (`PlayerKind::Goalie`) — drives the goalie-gear kit
    /// preview (goalie meshes + equipment) instead of the skater set.
    pub is_goalie: bool,
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
    /// Resolved, display-ready banner values for the Player detail header.
    pub header: PlayerHeader,
    /// Resolved contract/draft/rookie/junior fields for the Player Data tab.
    pub data: PlayerData,
    /// Labelled, ordered ratings + traits for the Attributes tab.
    pub attributes: crate::attributes::PlayerAttributes,
    /// The server-driven curated section layout the UI renders (evidence-first per-game profile).
    pub profile: crate::field_profile::EntityFieldProfile,
}

/// Resolved, display-ready values for the Player detail banner. Every field is already localized
/// (enum labels, formatted height/weight/date, resolved team) so the UI renders it verbatim — no
/// raw ids or arithmetic in the frontend (the human-readable-surfaces rule).
///
/// EA bio encodings (studio ids on `ubPc`, sourced from the prior app and to be re-verified against
/// our own data): height `ypBQ + 54` inches; weight `sNZW + 120` lb (raw 0 = "no weight" sentinel);
/// birthdate composed from `qFnd + 1900` plus a **probed** month/day offset (`JKLp`/`Kswi` are
/// 0-indexed on NHL 11→21 but already 1-indexed on NHL 10 — see `probe_birthdate_encoding`);
/// primary team `dXSB - 1`
/// (0 = free agent); style/archetype = `QgFs` on the ratings companion (`dSvy`/`mHuy`).
#[derive(Clone, Debug, Default)]
pub struct PlayerHeader {
    pub full_name: String,
    pub position_label: Option<String>,
    pub handedness_label: Option<String>,
    pub nationality_label: Option<String>,
    /// "City, State/Province/Country" — birth city joined with the resolved birth-state label.
    pub birthplace: Option<String>,
    /// Composed `YYYY-MM-DD`.
    pub birthdate: Option<String>,
    /// Formatted height, e.g. `6'1"`.
    pub height: Option<String>,
    /// Formatted weight, e.g. `190 lb`.
    pub weight: Option<String>,
    /// Player/goalie style (archetype) label.
    pub style_label: Option<String>,
    pub primary_team_id: Option<u64>,
    pub primary_team_name: Option<String>,
    pub jersey_number: Option<i64>,
}

/// Resolved "Player Data" tab fields beyond the header — contract, draft, rookie flags, and the
/// junior/pro (rights) team joins. Sourced from the prior app's operator-anchored field map (Tasks
/// #41–46), to be re-verified against our data. Raw studio ids on `ubPc`: cap hit `kKhd` × $5000,
/// term `IhDG` yrs, two-way `LoVD`, rights/pro team `dbBW`, junior `ZbNy`, draft team `vgWu`, round
/// `ccjU`, pick `tTfW`, year `YKzW` + 1900 (255 = undrafted), rookie NHL/AHL/CHL `dCnv`/`ywRO`/`BQVU`,
/// Year-UFA `GKKz` + 1900. Every 1-indexed team ref decodes `id - 1` (0 = none).
#[derive(Clone, Debug, Default)]
pub struct PlayerData {
    pub year_ufa: Option<u32>,
    pub pro_team_id: Option<u64>,
    pub pro_team_name: Option<String>,
    /// Formatted, e.g. `$5,000,000`.
    pub cap_hit: Option<String>,
    pub contract_length_years: Option<u32>,
    pub contract_status: Option<String>,
    pub is_two_way: Option<bool>,
    pub is_multi_year: Option<bool>,
    pub junior_team_id: Option<u64>,
    pub junior_team_name: Option<String>,
    pub draft_team_id: Option<u64>,
    pub draft_team_name: Option<String>,
    pub draft_round: Option<u32>,
    pub draft_position: Option<u32>,
    pub draft_year: Option<u32>,
    pub is_rookie_nhl: Option<bool>,
    pub is_rookie_ahl: Option<bool>,
    pub is_rookie_chl: Option<bool>,
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

/// Per-companion-table reader for the summary's derived ratings values: the rating descriptors
/// keyed by their rating-name **suffix** (era-portable) plus each one's display offset, and the
/// `playerstyle` (`QgFs`) descriptor for the archetype label.
struct CompanionReader<'a> {
    table: &'a Table,
    ratings: HashMap<String, (&'a FieldDescriptor, i64)>,
    style: Option<&'a FieldDescriptor>,
}

impl<'a> CompanionReader<'a> {
    fn build(db: &'a TdbFile, meta: &FieldMetaIndex, studio_table: &str) -> Option<Self> {
        let table = db.table_canonical(studio_table)?;
        let mut ratings = HashMap::new();
        for f in &table.fields {
            let Some(m) = meta.field(&table.id.raw(), &f.id.raw()) else {
                continue;
            };
            if let Some(off) =
                crate::attributes::rating_offset(studio_table, &m.name, m.range_low, m.range_high)
            {
                if let Some(suffix) = crate::attributes::rating_suffix(&m.name) {
                    ratings.insert(suffix.to_string(), (f, off));
                }
            }
        }
        // Modern `playerstyle` (`QgFs`) or NHL07 gen6 `p_style`/`g_style` (`mQoE`/`wJus`).
        let style = descriptor_by_name(meta, table, "playerstyle")
            .or_else(|| descriptor_by_name(meta, table, "p_style"))
            .or_else(|| descriptor_by_name(meta, table, "g_style"));
        Some(Self { table, ratings, style })
    }

    /// One rating's display value (raw + era offset) by suffix.
    fn display_value(&self, db: &TdbFile, rec: u32, suffix: &str) -> Option<f64> {
        let (f, off) = self.ratings.get(suffix)?;
        db.read_u64(self.table, rec, f)
            .map(|raw| (raw as i64 + off) as f64)
    }

    /// The style/archetype enum label of this record, when the vocabulary knows the value.
    fn style_label(&self, db: &TdbFile, enums: &EnumRegistry, rec: u32) -> Option<String> {
        let f = self.style?;
        let raw = db.read_u64(self.table, rec, f)? as i64;
        let stbl = self.table.id.canonical(db.endian);
        let sfld = f.id.canonical(db.endian);
        // TOML enum (modern) or the synthesized NHL07 style vocabulary (`mQoE`/`wJus`).
        enums.label(&stbl, &sfld, raw).map(|s| s.to_string()).or_else(|| {
            crate::attributes::synthetic_enum(&stbl, &sfld, crate::attributes::is_gen6_style_schema(db))
                .and_then(|opts| opts.into_iter().find(|(r, _)| *r == raw).map(|(_, l)| l))
        })
    }
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
    // Resolve the summary descriptors once, then read each player's bio record.
    let first_f = descriptor_by_name(meta, bio, "firstname");
    let last_f = descriptor_by_name(meta, bio, "lastname");
    let pos_f = descriptor_by_name(meta, bio, "position");
    let jersey_f = descriptor_by_name(meta, bio, "jersey");
    let active_f = descriptor_by_name(meta, bio, "active");
    let rights_f = descriptor_by_name(meta, bio, "proteam");
    // Contract cells (studio ids per the Player Data field map: cap hit `kKhd` × $5000, term
    // `IhDG`, two-way `LoVD`).
    let by_studio = |id: &str| bio.fields.iter().find(|f| f.id.canonical(db.endian) == id);
    let caphit_f = by_studio("kKhd");
    let term_f = by_studio("IhDG");
    let twoway_f = by_studio("LoVD");
    let skater = CompanionReader::build(db, meta, ids::SKATER_AI);
    let goalie = CompanionReader::build(db, meta, ids::GOALIE_AI);

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
    // Sign/sentinel-decoded integer read off the bio row.
    let read_dec = |f: Option<&FieldDescriptor>, rec: u32| -> Option<i64> {
        let f = f?;
        let raw = db.read_u64(bio, rec, f)?;
        Some(
            meta.field(&bio.id.raw(), &f.id.raw())
                .map(|m| m.decode_raw(raw))
                .unwrap_or(raw as i64),
        )
    };

    model
        .players
        .iter()
        .map(|p| {
            let position_label = pos_f
                .map(|f| read_field(db, meta, enums, bio, p.bio_record, f).display)
                .unwrap_or_default();
            // Kind-correct companion: style label + derived Overall (skaters via the ported
            // formula over display values; goalies from the stored `overall` rating).
            let (companion, companion_rec) = match p.kind {
                PlayerKind::Skater => (skater.as_ref(), p.skater_ai),
                PlayerKind::Goalie => (goalie.as_ref(), p.goalie_ai),
            };
            let (style_label, overall) = match (companion, companion_rec) {
                (Some(c), Some(rec)) => (
                    c.style_label(db, enums, rec),
                    match p.kind {
                        PlayerKind::Skater => Some(crate::attributes::skater_overall(
                            |suffix| c.display_value(db, rec, suffix),
                            p.position == tdb_relations::Position::Defense,
                        )),
                        PlayerKind::Goalie => {
                            c.display_value(db, rec, "overall").map(|v| v as i64)
                        }
                    },
                ),
                _ => (None, None),
            };
            let caphit = read_dec(caphit_f, p.bio_record);
            PlayerSummary {
                player_id: p.player_id,
                first_name: read_str(first_f, p.bio_record),
                last_name: read_str(last_f, p.bio_record),
                position_label,
                team_id: team_of.get(&p.player_id).copied(),
                jersey_number: read_dec(jersey_f, p.bio_record),
                style_label,
                is_active: read_dec(active_f, p.bio_record).map(|v| v != 0),
                overall,
                // 1-indexed team ref: 0 (and the -1 the field's range allows) = no rights held.
                rights_team_id: read_dec(rights_f, p.bio_record)
                    .filter(|&v| v > 0)
                    .map(|v| (v - 1) as u64),
                bio_record: p.bio_record,
                // Dollars only when the salary scale is confirmed for this title (see
                // `cap_hit_scale`); left `None` on titles whose encoding is unverified.
                cap_hit_dollars: cap_hit_scale(db, meta)
                    .and_then(|mult| caphit.filter(|&r| r > 0).map(|r| r as u64 * mult)),
                contract_years: read_dec(term_f, p.bio_record)
                    .filter(|&r| r > 0)
                    .map(|r| r as u32),
                contract_status: caphit.map(|r| {
                    if r > 0 {
                        "Signed / Restricted".to_string()
                    } else {
                        "Unrestricted".to_string()
                    }
                }),
                is_two_way: read_dec(twoway_f, p.bio_record).map(|r| r != 0),
                is_goalie: p.kind == PlayerKind::Goalie,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Career stats — the four stock per-player stat tables (skater/goalie × regular season/playoffs),
// joined by the 1-indexed `wBIz` key. Stat cells are full `FieldView`s so the UI can edit them
// through the normal audited `set_field` path.
// ---------------------------------------------------------------------------

/// The studio ids of the four stock career-stats tables.
pub const CAREER_SKATER_SEASON: &str = "tsSy"; // ySst  stockcareerseasonplayerstats
pub const CAREER_SKATER_PLAYOFFS: &str = "OAqO"; // OqAO  stockcareerplayoffplayerstats
pub const CAREER_GOALIE_SEASON: &str = "USfF"; // FfSU  stockcareerseasongoaliestats
pub const CAREER_GOALIE_PLAYOFFS: &str = "nCDp"; // pDCn  stockcareerplayoffgoaliestats

/// Skater stat columns as `(metadata name, header label)`, in display order (the reference layout).
/// Shoot-out/shooting-%/blocked-shot fields are omitted: all-zero in the shipped stock data.
const SKATER_COLUMNS: &[(&str, &str)] = &[
    ("gamesplayed", "GP"),
    ("goals", "G"),
    ("assists", "A"),
    ("points", "PTS"),
    ("plusminus", "+/-"),
    ("penaltyminutes", "PIM"),
    ("shots", "SOG"),
    ("powerplaygoals", "PPG"),
    ("shorthandedgoals", "SHG"),
    ("gamewinninggoals", "GWG"),
    ("gametyinggoals", "GTG"), // absent from the playoff table → skipped there
];

/// Goalie stat columns (both goalie tables share the shape). Shoot-out GA/SA omitted (all-zero).
const GOALIE_COLUMNS: &[(&str, &str)] = &[
    ("gamesplayed", "GP"),
    ("wins", "W"),
    ("losses", "L"),
    ("ties", "T"),
    ("shutouts", "SO"),
    ("goalsagainst", "GA"),
    ("shotsagainst", "SA"),
    ("savepercent", "SV%"),
    ("gaa", "GAA"),
    ("minutes", "MIN"),
    ("goals", "G"),
    ("assists", "A"),
    ("points", "PTS"),
    ("penaltyminutes", "PIM"),
];

/// The static shape of one career section: its kind key, title, table, and column plan.
struct CareerShape {
    kind: &'static str,
    title: &'static str,
    table: &'static str,
    columns: &'static [(&'static str, &'static str)],
}

/// The four section shapes, in default display order (skater first; the UI reorders for goalies).
const CAREER_SECTIONS: &[CareerShape] = &[
    CareerShape {
        kind: "skater_season",
        title: "Skater — Regular Season",
        table: CAREER_SKATER_SEASON,
        columns: SKATER_COLUMNS,
    },
    CareerShape {
        kind: "skater_playoffs",
        title: "Skater — Playoffs",
        table: CAREER_SKATER_PLAYOFFS,
        columns: SKATER_COLUMNS,
    },
    CareerShape {
        kind: "goalie_season",
        title: "Goalie — Regular Season",
        table: CAREER_GOALIE_SEASON,
        columns: GOALIE_COLUMNS,
    },
    CareerShape {
        kind: "goalie_playoffs",
        title: "Goalie — Playoffs",
        table: CAREER_GOALIE_PLAYOFFS,
        columns: GOALIE_COLUMNS,
    },
];

/// One column of a career-stats section: the studio field id + the header label.
#[derive(Clone, Debug)]
pub struct CareerColumn {
    pub field: String,
    pub label: String,
}

/// One season row: the row's identity (season/league/team) resolved for display, plus the stat
/// cells as editable [`FieldView`]s aligned with the section's columns (`None` = field absent).
#[derive(Clone, Debug)]
pub struct CareerRow {
    /// Record index in the section's table — the edit address for `set_field`.
    pub record: u32,
    /// `"2006-07"` (the stored year is the season-END year, raw `+ 1900`).
    pub season_label: String,
    /// Resolved league label (`NHL`/`AHL`), or the raw value if uncatalogued.
    pub league_label: String,
    /// The stored team code string (e.g. `COL`), as-is.
    pub team: String,
    pub cells: Vec<Option<FieldView>>,
}

/// One career-stats section (one stock table).
#[derive(Clone, Debug)]
pub struct CareerSection {
    /// Stable kind key: `skater_season` | `skater_playoffs` | `goalie_season` | `goalie_playoffs`.
    pub kind: String,
    pub title: String,
    /// Studio table id — the edit/add/delete address.
    pub table: String,
    pub columns: Vec<CareerColumn>,
    pub rows: Vec<CareerRow>,
}

/// A player's stock career stats: the four sections (absent tables yield empty sections).
#[derive(Clone, Debug, Default)]
pub struct CareerStats {
    pub sections: Vec<CareerSection>,
}

/// The career tables key rows by `wBIz` stored **1-indexed** (`range_low = -1`, offset storage):
/// raw `0` is the "no player" sentinel and raw `N` belongs to bio `wBIz = N - 1`. Anchored by the
/// prior app against real stat lines (e.g. Stastny bio `wBIz=5` → career rows raw `6`).
fn career_join_raw(player_id: u64) -> u64 {
    player_id + 1
}

/// Read a player's stock career stats across all four tables. Tables absent from this title's
/// schema yield sections with empty columns/rows. Rows sort newest-first (year desc, record asc).
pub fn career_stats(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    player_id: u64,
) -> CareerStats {
    let sections = CAREER_SECTIONS
        .iter()
        .map(|shape| career_section(db, meta, enums, player_id, shape))
        .collect();
    CareerStats { sections }
}

fn career_section(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    player_id: u64,
    shape: &CareerShape,
) -> CareerSection {
    let empty = CareerSection {
        kind: shape.kind.to_string(),
        title: shape.title.to_string(),
        table: shape.table.to_string(),
        columns: Vec::new(),
        rows: Vec::new(),
    };
    let Some(table) = db.table_canonical(shape.table) else {
        return empty;
    };
    let Some(join) = descriptor_by_name(meta, table, "game_id") else {
        return empty;
    };
    // Resolve every descriptor once; columns absent from this table's schema are skipped.
    let cols: Vec<(&str, &FieldDescriptor)> = shape
        .columns
        .iter()
        .filter_map(|(n, l)| descriptor_by_name(meta, table, n).map(|f| (*l, f)))
        .collect();
    let year_f = descriptor_by_name(meta, table, "year");
    let league_f = descriptor_by_name(meta, table, "league");
    let team_f = descriptor_by_name(meta, table, "teams");

    let wanted = career_join_raw(player_id);
    let mut rows: Vec<(i64, CareerRow)> = (0..table.current_records as u32)
        .filter(|&i| db.read_u64(table, i, join) == Some(wanted))
        .map(|i| {
            let year_end = year_f
                .and_then(|f| db.read_u64(table, i, f))
                .map(|raw| raw as i64 + 1900)
                .unwrap_or(0);
            let league_label = league_f
                .map(|f| read_field(db, meta, enums, table, i, f).display)
                .unwrap_or_default();
            let team = team_f
                .and_then(|f| db.read_inline_str(table, i, f))
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            let cells = cols
                .iter()
                .map(|(_, f)| Some(read_field(db, meta, enums, table, i, f)))
                .collect();
            let row = CareerRow {
                record: i,
                season_label: season_label(year_end),
                league_label,
                team,
                cells,
            };
            (year_end, row)
        })
        .collect();
    // Newest season first; equal years keep record order (matches the reference presentation).
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.record.cmp(&b.1.record)));

    CareerSection {
        columns: cols
            .iter()
            .map(|(l, f)| CareerColumn {
                field: f.id.canonical(db.endian),
                label: l.to_string(),
            })
            .collect(),
        rows: rows.into_iter().map(|(_, r)| r).collect(),
        ..empty
    }
}

/// `2007` → `"2006-07"` (the stored year is the season-end year).
fn season_label(year_end: i64) -> String {
    if !(1901..=2155).contains(&year_end) {
        return year_end.to_string();
    }
    format!("{}-{:02}", year_end - 1, year_end % 100)
}

// ---------------------------------------------------------------------------
// Lines / depth chart view-model
// ---------------------------------------------------------------------------

/// One roster entry on a team's depth chart: the player + per-slot situational assignments.
#[derive(Clone, Debug)]
pub struct LineEntry {
    pub slot_record: u32,
    pub player_id: Option<u64>,
    pub name: String,
    pub position_label: String,
    pub jersey: Option<u64>,
    pub captaincy: Option<u64>,
    /// Lineup status from `eGlu.hSZj`: `Some(true)` dressed, `Some(false)` scratched (Spike S4).
    pub dressed: Option<bool>,
    /// Studio field ids of the 1-bit situational flags set on this `eGlu` row (see
    /// [`LinesView::slot_fields`] for the id → name catalog).
    pub slots: Vec<String>,
    /// The per-slot archetype cell (`eGlu.QgFs`), when the schema carries it. Spike S4 confirmed
    /// this binding: the anchor distribution matches the skater `dSvy.QgFs` style enum (1-based),
    /// so it renders through the shared confirmed Player Style vocabulary.
    pub style: Option<FieldView>,
}

/// One situational slot field on `eGlu`: the studio id plus its metadata name (`l1c`, `pp1lw`,
/// `ot_2rd`, `s1`, `x1`, …) — the name encodes group + position, so the UI derives the whole
/// lines layout from this catalog.
#[derive(Clone, Debug)]
pub struct SlotField {
    pub studio_field: String,
    pub name: String,
}

/// A team's depth chart: its roster entries with situational assignments.
#[derive(Clone, Debug)]
pub struct LinesView {
    pub team_id: u64,
    /// Every 1-bit situational slot field on this title's `eGlu`, in schema order.
    pub slot_fields: Vec<SlotField>,
    pub entries: Vec<LineEntry>,
}

/// Player display name ("Last, First") + position label, by `wBIz`.
fn player_name_pos(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    model: &RosterModel,
    player_id: u64,
) -> (String, String) {
    let Some(p) = model.player(player_id) else {
        return (String::new(), String::new());
    };
    let Some(bio) = db.table_canonical(ids::BIO) else {
        return (String::new(), String::new());
    };
    let first = read_str_by_name(db, meta, bio, p.bio_record, "firstname");
    let last = read_str_by_name(db, meta, bio, p.bio_record, "lastname");
    let name = if first.is_empty() {
        last
    } else {
        format!("{last}, {first}")
    };
    let position_label = descriptor_by_name(meta, bio, "position")
        .map(|f| read_field(db, meta, enums, bio, p.bio_record, f).display)
        .unwrap_or_default();
    (name, position_label)
}

/// Build a team's depth chart: each roster slot's player plus the set 1-bit situational flags,
/// labelled from the metadata field names.
pub fn lines_view(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    model: &RosterModel,
    team_id: u64,
) -> Option<LinesView> {
    model.team(team_id)?;
    let eglu = db.table_canonical(ids::ROSTER_LINES)?;

    // The 1-bit situational flag descriptors, paired with their metadata name. `rosterstatus`
    // (a 1-bit non-slot flag) is excluded — only the named line/unit slots belong here.
    let flags: Vec<(&str, &FieldDescriptor)> = eglu
        .fields
        .iter()
        .filter(|f| f.bit_width == 1)
        .filter_map(|f| {
            meta.field(&eglu.id.raw(), &f.id.raw())
                .filter(|m| !m.name.is_empty() && m.name != "rosterstatus")
                .map(|m| (m.name.as_str(), f))
        })
        .collect();
    let slot_fields = flags
        .iter()
        .map(|(name, f)| SlotField {
            studio_field: f.id.canonical(db.endian),
            name: name.to_string(),
        })
        .collect();
    let style_f = eglu
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "QgFs");

    let entries = model
        .team_roster(team_id)
        .iter()
        .map(|slot| {
            let set_slots = flags
                .iter()
                .filter(|(_, f)| db.read_u64(eglu, slot.record, f) == Some(1))
                .map(|(_, f)| f.id.canonical(db.endian))
                .collect();
            let (name, position_label) = slot
                .player_id
                .map(|pid| player_name_pos(db, meta, enums, model, pid))
                .unwrap_or_default();
            LineEntry {
                slot_record: slot.record,
                player_id: slot.player_id,
                name,
                position_label,
                jersey: slot.jersey,
                captaincy: slot.captaincy,
                dressed: slot.dressed,
                slots: set_slots,
                style: style_f.map(|f| read_field(db, meta, enums, eglu, slot.record, f)),
            }
        })
        .collect();

    Some(LinesView {
        team_id,
        slot_fields,
        entries,
    })
}

// ---------------------------------------------------------------------------
// Team & arena view-models
// ---------------------------------------------------------------------------

/// Studio ids for the arena link (verified: `kOtt.buDa → StEO.vfEq`, audited 2026-05-24).
const ARENA_TABLE: &str = "StEO";
const ARENA_KEY: &str = "vfEq";
const TEAM_ARENA_FK: &str = "buDa";
/// `kOtt.lOnr` — the team's logo **art id** (EA `artid`), which names the on-disk
/// logo texlib. Distinct from the `ubPc.lOnr` portrait id (same raw id, other table).
const TEAM_LOGO_ARTID: &str = "lOnr";

/// Curated team identity fields (metadata `name`s), in display order.
const TEAM_IDENTITY: &[&str] = &[
    "fullname",
    "abbrname",
    "shortname",
    "artabbr",
    "arenaid",
    "leaguegroup",
    "divisiongroup",
    "conferencegroup",
];

/// Curated arena identity fields (metadata `name`s), in display order.
const ARENA_IDENTITY: &[&str] = &["arenaname", "cityname", "arenatype", "maxattendance"];

/// A compact team row for the master list.
#[derive(Clone, Debug)]
pub struct TeamSummary {
    pub team_id: u64,
    pub record: u32,
    pub name: String,
    pub abbrev: String,
    /// League code (`kOtt.xMjj`) + its resolved label — powers league-scoped team pickers.
    pub league: Option<i64>,
    pub league_label: Option<String>,
}

/// A resolved arena reference (the team's home rink).
#[derive(Clone, Debug)]
pub struct ArenaRef {
    pub arena_id: u64,
    pub name: String,
    pub city: String,
}

/// The full Team detail: curated identity, resolved arena, roster size, and all `kOtt` fields.
#[derive(Clone, Debug)]
pub struct TeamView {
    pub team_id: u64,
    pub record: u32,
    pub identity: Vec<FieldView>,
    pub arena: Option<ArenaRef>,
    pub roster_count: usize,
    /// This team's retired jersey numbers (`wcSL`), sorted ascending.
    pub retired_numbers: Vec<u32>,
    pub all_fields: Vec<FieldView>,
    /// The team's logo art id (`kOtt.lOnr`) — names the on-disk logo texlib. `None`
    /// on schemas without the field. Kept for the header logo preview; the raw id
    /// stays behind the scenes (human-readable-surfaces rule).
    pub logo_artid: Option<u64>,
    /// The server-driven curated section layout the UI renders (evidence-first per-game profile).
    pub profile: crate::field_profile::EntityFieldProfile,
}

/// A compact arena row for the master list.
#[derive(Clone, Debug)]
pub struct ArenaSummary {
    pub arena_id: u64,
    pub record: u32,
    pub name: String,
    pub city: String,
    /// How many teams call this arena home (`kOtt.buDa == arena_id`).
    pub team_count: u32,
}

/// The full Arena detail: curated identity, the resolved tenants, plus all `StEO` fields.
#[derive(Clone, Debug)]
pub struct ArenaView {
    pub arena_id: u64,
    pub record: u32,
    pub identity: Vec<FieldView>,
    /// Teams whose home arena this is (`kOtt.buDa == arena_id`), resolved to names.
    pub teams: Vec<TeamSummary>,
    pub all_fields: Vec<FieldView>,
}

/// Read a fixed-bit field's raw value by studio id from one record of a table.
pub(crate) fn read_u64_by_studio(
    db: &TdbFile,
    table: &Table,
    index: u32,
    studio_field: &str,
) -> Option<u64> {
    let f = table
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == studio_field)?;
    db.read_u64(table, index, f)
}

/// Read an inline string by metadata `name` from one record of a table.
fn read_str_by_name(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    table: &Table,
    index: u32,
    name: &str,
) -> String {
    descriptor_by_name(meta, table, name)
        .and_then(|f| db.read_inline_str(table, index, f))
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .unwrap_or_default()
}

/// Pick curated fields (by metadata `name`, in the given order) from a record's full field list.
fn curate(all_fields: &[FieldView], names: &[&str]) -> Vec<FieldView> {
    names
        .iter()
        .filter_map(|name| all_fields.iter().find(|f| &f.name == name).cloned())
        .collect()
}

// --- Player header helpers -------------------------------------------------

/// Find a [`FieldView`] by metadata `name`.
fn fv_name<'a>(fields: &'a [FieldView], name: &str) -> Option<&'a FieldView> {
    fields.iter().find(|f| f.name == name)
}

/// Find a [`FieldView`] by studio (canonical) field id.
fn fv_id<'a>(fields: &'a [FieldView], studio_field: &str) -> Option<&'a FieldView> {
    fields.iter().find(|f| f.studio_field == studio_field)
}

/// A non-empty enum label for the given field, if resolved.
fn label_of(f: Option<&FieldView>) -> Option<String> {
    f.and_then(|f| f.enum_label.clone()).filter(|s| !s.is_empty())
}

// Rating classification (which companion fields are 36..99/0..99 ratings, and their era-correct
// scale) lives in [`crate::attributes::rating_offset`], driven by the field's own metadata name +
// declared range — see the attributes module docs. `read_field` applies it below so the display
// and the Attributes-tab labels classify fields identically and never drift.

/// The base year added to a stored `ubPc.qFnd` (birth year), probed from the field's own bit width.
///
/// NHL 09→21 store the birth year in a **full 8-bit** field with base **1900** (raw 0..255 →
/// 1900..2155). The two earliest Xbox-360 titles pack it into a **narrower** field and rebase to
/// **1959** so the reduced range still spans a modern roster:
/// * **NHL07** — **6-bit** (raw 0..63), verified against six known players (Alfredsson 1972, Iginla
///   1977, Sakic 1969, Lidström 1970, Crosby 1987, Jágr 1972 → raw 0..29 → 1959..1988).
/// * **NHL08** — **7-bit** (raw 0..30 across all 2962 bio rows → base 1959 = 1959..1989; base 1900
///   would give an impossible 1900..1930 for a 2007-08 roster).
///
/// So any sub-8-bit `qFnd` uses base 1959; a full byte uses 1900. Keyed on the field width, not the
/// title id, per the version-aware / evidence-first rule.
fn birth_year_base(db: &TdbFile, bio: &Table) -> i64 {
    match bio.fields.iter().find(|f| f.id.canonical(db.endian) == "qFnd") {
        Some(f) if f.bit_width < 8 => 1959,
        _ => 1900,
    }
}

/// The display/edit offset for 6-bit (`0..63`) skater/goalie ratings, **probed from the companion
/// data** — the same evidence-first rule as the birthdate/main-team probes:
/// * **NHL 10 → Legacy** store ratings `14..63` shown on the **36..99** scale (offset **+36**).
/// * **NHL07 (gen6)** stores them `0..49` shown on the **50..99** scale (offset **+50**) — verified
///   in-game (our tool was 14 low vs the game) and by the anchor: every rating maxes at 49.
///
/// The two scales' stored ranges are disjoint, so a single question settles it: does *any* 6-bit
/// rating exceed 49? If so it's the modern +36 scale; if none do, it's gen6 +50. Early-exits on the
/// first >49 value, so modern DBs (a 63-rated star appears immediately) resolve in a few reads. PS4's
/// 7-bit `0..99` ratings never reach this path (they carry no offset).
fn sixbit_rating_offset(db: &TdbFile) -> i64 {
    for tbl in [ids::SKATER_AI, ids::GOALIE_AI] {
        let Some(t) = db.table_canonical(tbl) else { continue };
        for f in t.fields.iter().filter(|f| f.bit_width == 6) {
            for rec in 0..t.current_records as u32 {
                if db.read_u64(t, rec, f).unwrap_or(0) > 49 {
                    return 36; // a rating in 50..63 exists → the modern scale
                }
            }
        }
    }
    50 // no 6-bit rating exceeds 49 → the gen6 (NHL07) 50..99 scale
}

/// EA-semantic human formatting for known numeric fields, applied at [`read_field`] so the value
/// reads correctly everywhere a `FieldView.display` is shown — the Bio all-fields list, the Ratings
/// tab, career, and the header alike, not just one panel. Sourced from the prior app's field map
/// (verify against real data). Returns `None` for fields with no special encoding (plain number).
fn ea_semantic_display(
    studio_table: &str,
    studio_field: &str,
    decoded: i64,
    birthdate: impl Fn() -> BirthdateEncoding,
    birth_year_base: impl Fn() -> i64,
    draft_year_base: impl Fn() -> Option<i64>,
    weight_base: impl Fn() -> i64,
) -> Option<String> {
    match (studio_table, studio_field) {
        ("ubPc", "ypBQ") => format_height(decoded), // height: inches = raw + 54
        ("ubPc", "sNZW") => format_weight(decoded, weight_base()), // weight: lb = raw + base (0 = unset)
        ("ubPc", "qFnd") => Some((decoded + birth_year_base()).to_string()), // birth year (base probed)
        // Month/day indexing drifts across titles — NHL 10 stores them already
        // 1-indexed while NHL 11→21 store them 0-indexed, and the metadata is
        // identical in both. `birthdate` is a closure so the probe only runs for
        // these two fields, never for the rest of the match.
        ("ubPc", "JKLp" | "Kswi") => Some((decoded + birthdate().offset()).to_string()),
        // Year of UFA eligibility = raw + 1900; guard the plausible range, else leave raw.
        ("ubPc", "GKKz") => (90..=130).contains(&decoded).then(|| (decoded + 1900).to_string()),
        // Draft year: modern `draftyear` (`YKzW`, base 1900, 255 = undrafted) or NHL07 gen6
        // `yeardrafted` (`ygQP`, base 1959, raw 0..46 = 1959..2005; anything above is "unavailable").
        // The base is schema-probed; when unconfirmed the value stays raw.
        ("ubPc", "YKzW" | "ygQP") => draft_year_base().and_then(|base| {
            let undrafted = decoded == 255 || (base == 1959 && !(0..=46).contains(&decoded));
            (!undrafted).then(|| (decoded + base).to_string())
        }),
        // Skater/goalie ratings are handled in `read_field` via `attributes::rating_offset`
        // (metadata-classified, era-correct scale) — not here, which only sees ids.
        // Career tables: season-end year; signed plus/minus; fixed-point SV% (×0.001) and GAA (×0.01).
        (t, "qFnd") if is_career_table(t) => Some((decoded + 1900).to_string()),
        ("tsSy" | "OAqO", "Uomt") => Some(if decoded == 0 {
            "0".to_string()
        } else {
            format!("{decoded:+}")
        }),
        ("USfF" | "nCDp", "ZMAR") => Some(format_savepct(decoded)),
        ("USfF" | "nCDp", "beSh") => Some(format!("{:.2}", decoded as f64 / 100.0)),
        _ => None,
    }
}

/// `true` for the four stock career-stats tables (studio ids).
fn is_career_table(studio_table: &str) -> bool {
    matches!(studio_table, "tsSy" | "OAqO" | "USfF" | "nCDp")
}

/// Save percentage is stored ×1000 (`914` = .914). The reference presentation drops the leading
/// zero (`.914`); a perfect 1000 renders `1.000`.
fn format_savepct(decoded: i64) -> String {
    let s = format!("{:.3}", decoded as f64 / 1000.0);
    s.strip_prefix('0').map(str::to_string).unwrap_or(s)
}

/// The friendly-edit transform for a field: `(offset, is_height)`. The front-facing value is
/// `decoded + offset` (e.g. weight lb, birth year, rating 36..99), so an editor can accept the human
/// number and stage `input - offset`. Height additionally presents/parses that value as `ft'in"`.
/// The offsets mirror [`ea_semantic_display`] so read and write stay symmetric in one place.
fn ea_edit_form(
    studio_table: &str,
    studio_field: &str,
    birthdate: impl Fn() -> BirthdateEncoding,
    birth_year_base: impl Fn() -> i64,
    draft_year_base: impl Fn() -> Option<i64>,
    weight_base: impl Fn() -> i64,
) -> (Option<i64>, bool) {
    match (studio_table, studio_field) {
        ("ubPc", "ypBQ") => (Some(54), true),                        // height (inches = raw + 54)
        ("ubPc", "sNZW") => (Some(weight_base()), false),            // weight lb (base probed)
        ("ubPc", "qFnd") => (Some(birth_year_base()), false),        // birth year (base probed)
        ("ubPc", "GKKz") => (Some(1900), false),                     // UFA year
        ("ubPc", "YKzW" | "ygQP") => (draft_year_base(), false),     // draft year (base probed)
        // Per-title indexing — see `ea_semantic_display`. Staging `input - offset`
        // with the wrong offset would write a date a month and a day off, so this
        // must use the same probe the display does.
        ("ubPc", "JKLp" | "Kswi") => (Some(birthdate().offset()), false),
        (t, "qFnd") if is_career_table(t) => (Some(1900), false), // career season-end year
        _ => (None, false),
    }
}

/// The fixed-point scale for a field: the front-facing value is `decoded * scale` (`.914`, `2.43`)
/// and an editor stages `round(input / scale)`. `None` for plain integer fields. Mirrors the
/// [`ea_semantic_display`] SV%/GAA formatting so read and write stay symmetric.
fn ea_edit_scale(studio_table: &str, studio_field: &str) -> Option<f64> {
    match (studio_table, studio_field) {
        ("USfF" | "nCDp", "ZMAR") => Some(0.001), // save % stored ×1000
        ("USfF" | "nCDp", "beSh") => Some(0.01),  // GAA stored ×100
        _ => None,
    }
}

/// EA height decode: raw + 54 = inches → `ft'in"`. Sane human range 48..96 else `None`.
fn format_height(decoded: i64) -> Option<String> {
    let inches = decoded + 54;
    if !(48..=96).contains(&inches) {
        return None;
    }
    Some(format!("{}'{}\"", inches / 12, inches % 12))
}

/// EA weight decode: raw + `base` = pounds (base +120 modern, +150 NHL07 gen6); raw <= 0 is the
/// "no weight set" sentinel → `None`.
fn format_weight(decoded: i64, base: i64) -> Option<String> {
    (decoded > 0).then(|| format!("{} lb", decoded + base))
}

/// Compose `YYYY-MM-DD` from the EA birthdate fields (year = raw+1900, month/day are 0-indexed).
fn compose_birthdate(
    year: Option<i64>,
    month: Option<i64>,
    day: Option<i64>,
    birthdate: BirthdateEncoding,
    year_base: i64,
) -> Option<String> {
    let off = birthdate.offset();
    let (y, m, d) = (year? + year_base, month? + off, day? + off);
    if !(1900..=2100).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// How `ubPc.dXSB` (the player's current / "main" team pointer) encodes the `kOtt.vfEq` team id.
/// The convention drifted across the title range and is **probed from data**, never assumed — the
/// same evidence-first rule as [`crate::probe_birthdate_encoding`]:
///
/// | Titles                    | `dXSB` raw | Free agent |
/// | ------------------------- | ---------- | ---------- |
/// | NHL 09 / 10 (early Xbox360)| team id (0-indexed)        | (unrostered) |
/// | NHL Legacy / 11→21        | team id + 1 (1-indexed)    | raw `0` |
///
/// The `eGlu` roster is authoritative for where a player actually plays, so the offset is recovered
/// by asking — across rostered players — whether `dXSB` or `dXSB − 1` lands on a team the player is
/// truly a member of. Verified: NHL 09 & NHL 10 agree at offset 0 for every rostered player (0 at
/// offset 1); NHL Legacy agrees at offset 1 for all 5110 (0 at offset 0). Note the "pro team" ref
/// `dbBW` stays 1-indexed on every title — only `dXSB` drifted, so only `dXSB` is probed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainTeamEncoding {
    /// `dXSB` raw is the team id directly — NHL 09 / 10.
    ZeroIndexed,
    /// `dXSB` raw is `team id + 1`, `0` = free agent — NHL Legacy / 11→21.
    OneIndexed,
}

impl MainTeamEncoding {
    /// What to subtract from the raw `dXSB` to get the `kOtt.vfEq` team id.
    pub fn offset(self) -> u64 {
        match self {
            MainTeamEncoding::ZeroIndexed => 0,
            MainTeamEncoding::OneIndexed => 1,
        }
    }
}

/// Probe the `dXSB` main-team encoding from roster ground truth (see [`MainTeamEncoding`]). Falls
/// back to `OneIndexed` (the Legacy/modern convention) when the db carries no rostered players to
/// decide from.
pub fn probe_main_team_encoding(db: &TdbFile, model: &RosterModel) -> MainTeamEncoding {
    let Some(bio) = db.table_canonical(ids::BIO) else {
        return MainTeamEncoding::OneIndexed;
    };
    let Some(dxsb) = bio.fields.iter().find(|f| f.id.canonical(db.endian) == "dXSB") else {
        return MainTeamEncoding::OneIndexed;
    };
    // Membership set per player (the teams whose roster the player actually fills).
    let mut memb: HashMap<u64, std::collections::HashSet<u64>> = HashMap::new();
    for s in &model.slots {
        if let Some(pid) = s.player_id {
            memb.entry(pid).or_default().insert(s.team_id);
        }
    }
    let (mut off0, mut off1) = (0u32, 0u32);
    for p in &model.players {
        let Some(set) = memb.get(&p.player_id) else { continue };
        let Some(raw) = db.read_u64(bio, p.bio_record, dxsb) else { continue };
        if set.contains(&raw) {
            off0 += 1;
        }
        if raw > 0 && set.contains(&(raw - 1)) {
            off1 += 1;
        }
        if off0 + off1 >= 128 {
            break; // a decisive sample — the two conventions never tie in practice
        }
    }
    if off0 > off1 {
        MainTeamEncoding::ZeroIndexed
    } else {
        MainTeamEncoding::OneIndexed
    }
}

/// Where a team's display-name fields (`fullname`/`abbrname`/`shortname`) actually live.
///
/// NHL 09→21 carry them inline on `kOtt` (`exhibitionteams` is the fat team table). NHL07's `kOtt`
/// is a *thin* index whose `teamid` + `table` discriminator point into the stock/custom team tables,
/// which hold the names (verified: `table` is a 1-bit field, `0` = `stockteamtable`, `1` =
/// `customteamtable`; vanilla ships all 93 rows on stock). Returns the table + its studio id + the
/// record to read the name fields from; falls back to `kOtt` itself if the join can't be followed
/// (yields the same empty-name behavior as before, never an error).
struct TeamNameSource<'a> {
    table: &'a Table,
    studio_id: &'static str,
    record: u32,
}

fn team_name_source<'a>(
    db: &'a TdbFile,
    meta: &FieldMetaIndex,
    kott: &'a Table,
    kott_record: u32,
) -> TeamNameSource<'a> {
    let fat = TeamNameSource { table: kott, studio_id: ids::TEAM, record: kott_record };
    // Fat shape: the name fields are on kOtt itself.
    if descriptor_by_name(meta, kott, "fullname").is_some() {
        return fat;
    }
    // Thin shape (NHL07): follow (table discriminator, teamid) into the name-bearing table.
    let resolved = (|| {
        let teamid = read_u64_by_studio(db, kott, kott_record, ids::TEAM_NAME_ID)?;
        let disc = read_u64_by_studio(db, kott, kott_record, ids::TEAM_NAME_TABLE_DISC).unwrap_or(0);
        let studio_id = if disc == 1 { ids::CUSTOM_TEAM } else { ids::STOCK_TEAM };
        let table = db.table_canonical(studio_id)?;
        let key = descriptor_by_name(meta, table, "index")?;
        let record = (0..table.current_records as u32).find(|&i| db.read_u64(table, i, key) == Some(teamid))?;
        Some(TeamNameSource { table, studio_id, record })
    })();
    resolved.unwrap_or(fat)
}

/// A team's full name by id, via the model's `kOtt` record (following the NHL07 thin-index join).
pub(crate) fn team_name_by_id(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    model: &RosterModel,
    team_id: u64,
) -> Option<String> {
    let team = model.team(team_id)?;
    let kott = db.table_canonical(ids::TEAM)?;
    let src = team_name_source(db, meta, kott, team.record);
    let name = read_str_by_name(db, meta, src.table, src.record, "fullname");
    (!name.is_empty()).then_some(name)
}

/// Group an integer with thousands separators, e.g. `5000000` → `5,000,000`.
fn group_thousands(n: u64) -> String {
    let s = n.to_string();
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in b.iter().enumerate() {
        if i > 0 && (b.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*ch as char);
    }
    out
}

/// The multiplier that turns the stored `contractdollars` value into real dollars, or `None` when
/// the scale is **not confirmed** for the loaded title (the caller then shows the raw number and
/// flags it). Evidence-first, keyed on the field's bit width — never the title id:
///
/// `contractdollars` widens across the range (NHL07 9-bit → max 511, NHL10 10-bit, NHL14/Legacy
/// 13-bit → max 8191). The `× $5000` scale only spans a realistic salary ceiling on the 13-bit
/// field ($40.9M); on the narrow gen6/early fields it caps out at ~$2.5M, far below period cap
/// hits, so the multiplier there is different and unrecovered — we decline to fabricate a dollar
/// figure and mark the cell unverified instead.
fn cap_hit_scale(db: &TdbFile, meta: &FieldMetaIndex) -> Option<u64> {
    let bio = db.table_canonical(ids::BIO)?;
    let f = descriptor_by_name(meta, bio, "contractdollars")?;
    match f.bit_width {
        // Modern 13-bit `contractdollars` (NHL 14/15/Legacy): ×$5000 (max $40.9M).
        w if w >= 13 => Some(5000),
        // NHL07 gen6 9-bit `contractdollars`: ×$25,000, $25k increments (verified in-game — Bryzgalov
        // raw 40 = $1M, Giguère raw 160 = $4M; max 511 = $12.775M).
        9 => Some(25_000),
        _ => None,
    }
}

/// The base year added to the stored draft year, or `None` when it is **not confirmed** for the
/// loaded title. Only the modern `draftyear` field (8-bit, `+1900`, `255` = undrafted) is verified;
/// the legacy `yeardrafted` field (NHL07 6-bit, NHL10 8-bit) has an unconfirmed base, so its cell
/// shows the raw stored value and is flagged rather than mislabelled as a calendar year.
fn draft_year_base(db: &TdbFile, meta: &FieldMetaIndex) -> Option<i64> {
    let bio = db.table_canonical(ids::BIO)?;
    // Modern `draftyear` (8-bit) → +1900.
    if descriptor_by_name(meta, bio, "draftyear").is_some() {
        return Some(1900);
    }
    // NHL07 gen6 `yeardrafted` is a 6-bit field based at 1959 (verified in-game: Bryzgalov raw 41 =
    // 2000, Giguère raw 36 = 1995, Guerin raw 30 = 1989 — raw 0..46 → 1959..2005, plus an
    // "unavailable" sentinel). The wider (8-bit) `yeardrafted` on NHL10 keeps an unconfirmed base.
    let f = descriptor_by_name(meta, bio, "yeardrafted")?;
    (f.bit_width <= 6).then_some(1959)
}

/// The Stage-4 "unverified" verdicts, resolved per curated cell label. Kept as a free function so
/// tests can exercise the same rules the profile renders by.
fn unverified_note(db: &TdbFile, meta: &FieldMetaIndex, label: &str) -> Option<String> {
    match label {
        "Cap Hit" => cap_hit_scale(db, meta).is_none().then(|| {
            "salary scale unconfirmed for this game — showing the raw contract value".to_string()
        }),
        "Draft Year" => draft_year_base(db, meta).is_none().then(|| {
            "draft-year base unconfirmed for this game — showing the raw stored value".to_string()
        }),
        _ => None,
    }
}

/// Assemble the resolved "Player Data" tab fields (contract/draft/rookie/junior/pro) from the bio
/// field views, resolving 1-indexed team refs against the model.
fn build_player_data(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    model: &RosterModel,
    all_fields: &[FieldView],
) -> PlayerData {
    let dec = |id: &str| fv_id(all_fields, id).and_then(|f| f.decoded);
    // Resolve a 1-indexed team ref (`id - 1`; 0/None = none) to (id, name).
    let team = |raw: Option<i64>| -> (Option<u64>, Option<String>) {
        match raw {
            Some(v) if v > 0 => {
                let id = (v - 1) as u64;
                (Some(id), team_name_by_id(db, meta, model, id))
            }
            _ => (None, None),
        }
    };

    let (pro_team_id, pro_team_name) = team(dec("dbBW"));
    let (junior_team_id, junior_team_name) = team(dec("ZbNy"));
    let (draft_team_id, draft_team_name) = team(dec("vgWu"));
    let kkhd = dec("kKhd");
    let ihdg = dec("IhDG");

    PlayerData {
        year_ufa: dec("GKKz")
            .filter(|r| (90..=130).contains(r))
            .map(|r| (r + 1900) as u32),
        pro_team_id,
        pro_team_name,
        // Dollars only when the salary scale is confirmed for this title; otherwise the raw stored
        // value (the profile flags the cell unverified). Keyed on the field, not the title id.
        cap_hit: kkhd.filter(|&r| r > 0).map(|r| match cap_hit_scale(db, meta) {
            Some(mult) => format!("${}", group_thousands(r as u64 * mult)),
            None => r.to_string(),
        }),
        contract_length_years: ihdg.filter(|&r| r > 0).map(|r| r as u32),
        // Only meaningful when the cap-hit field is present; derived from whether a cap hit exists.
        contract_status: kkhd.map(|r| {
            if r > 0 {
                "Signed / Restricted".to_string()
            } else {
                "Unrestricted".to_string()
            }
        }),
        is_two_way: dec("LoVD").map(|r| r != 0),
        is_multi_year: ihdg.map(|r| r > 1),
        junior_team_id,
        junior_team_name,
        draft_team_id,
        draft_team_name,
        draft_round: dec("ccjU").filter(|&r| r > 0).map(|r| r as u32),
        draft_position: dec("tTfW").filter(|&r| r > 0).map(|r| r as u32),
        draft_year: dec("YKzW").filter(|&r| r != 255).map(|r| (r + 1900) as u32),
        is_rookie_nhl: dec("dCnv").map(|r| r != 0),
        is_rookie_ahl: dec("ywRO").map(|r| r != 0),
        is_rookie_chl: dec("BQVU").map(|r| r != 0),
    }
}

/// Assemble the resolved Player header from the already-read bio + ratings field views.
fn build_player_header(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    model: &RosterModel,
    all_fields: &[FieldView],
    ratings: &[FieldView],
    memberships: &[RosterMembership],
) -> PlayerHeader {
    let first = fv_name(all_fields, "firstname").map(|f| f.display.trim().to_string());
    let last = fv_name(all_fields, "lastname").map(|f| f.display.trim().to_string());
    let full_name = match (first.as_deref().unwrap_or(""), last.as_deref().unwrap_or("")) {
        ("", l) => l.to_string(),
        (f, "") => f.to_string(),
        (f, l) => format!("{f} {l}"),
    };

    let birth_city = fv_id(all_fields, "MFzJ")
        .map(|f| f.display.trim().to_string())
        .filter(|s| !s.is_empty());
    // The birth-state vocabulary is country-first verbatim ("Canada, Quebec" — it groups the
    // dropdown by country). For the banner's prose, flip it so the birthplace reads smallest to
    // largest: "City, Quebec, Canada". Single-word labels (bare countries) pass through.
    let birth_state = label_of(fv_id(all_fields, "QvxI")).map(|label| {
        match label.split_once(", ") {
            Some((country, state)) => format!("{state}, {country}"),
            None => label,
        }
    });
    let birthplace = match (birth_city, birth_state) {
        (Some(c), Some(s)) => Some(format!("{c}, {s}")),
        (Some(c), None) => Some(c),
        (None, Some(s)) => Some(s),
        (None, None) => None,
    };

    // Primary ("main") team from `ubPc.dXSB`, decoded with the **probed** per-title offset (NHL
    // 09/10 store it 0-indexed; Legacy/11→21 1-indexed with 0 = free agent — see
    // `MainTeamEncoding`). The `eGlu` roster is the ground truth for where a player plays, so the
    // resolved id is anchored on it: prefer the team `dXSB` points at when the player is actually a
    // member of it, else the first membership, else free agent. This keeps the banner, the
    // "Additional Teams" split, and the audit all agreeing on one main team across the title range.
    let encoding = probe_main_team_encoding(db, model);
    let dxsb = fv_id(all_fields, "dXSB").and_then(|f| f.decoded);
    let member_ids: Vec<u64> = memberships.iter().map(|m| m.team_id).collect();
    let dxsb_team = match (encoding, dxsb) {
        // 1-indexed FA sentinel.
        (MainTeamEncoding::OneIndexed, Some(0)) => None,
        (MainTeamEncoding::OneIndexed, Some(v)) if v > 0 => Some((v - 1) as u64),
        (MainTeamEncoding::ZeroIndexed, Some(v)) => Some(v as u64),
        _ => None,
    };
    let primary_team_id = match dxsb_team {
        // Trust `dXSB` when it names a team the player actually fills a slot on.
        Some(id) if member_ids.contains(&id) => Some(id),
        // `dXSB` resolved but the player isn't on that roster (edited/edge data): fall back to the
        // real membership so the banner never shows a team the player doesn't play for.
        Some(id) if member_ids.is_empty() => Some(id),
        Some(_) => member_ids.first().copied(),
        None => member_ids.first().copied(),
    };
    let primary_team_name = primary_team_id.and_then(|id| team_name_by_id(db, meta, model, id));

    PlayerHeader {
        full_name,
        position_label: label_of(fv_name(all_fields, "position")),
        handedness_label: label_of(fv_name(all_fields, "handedness")),
        nationality_label: label_of(fv_name(all_fields, "intlcountry")),
        birthplace,
        birthdate: compose_birthdate(
            fv_id(all_fields, "qFnd").and_then(|f| f.decoded),
            fv_id(all_fields, "JKLp").and_then(|f| f.decoded),
            fv_id(all_fields, "Kswi").and_then(|f| f.decoded),
            db.table_canonical(ids::BIO)
                .map(|bio| probe_birthdate_encoding(db, bio))
                .unwrap_or(BirthdateEncoding::ZeroIndexed),
            db.table_canonical(ids::BIO).map(|bio| birth_year_base(db, bio)).unwrap_or(1900),
        ),
        height: fv_name(all_fields, "height").and_then(|f| f.decoded).and_then(format_height),
        weight: fv_name(all_fields, "weight")
            .and_then(|f| f.decoded)
            .and_then(|d| format_weight(d, if sixbit_rating_offset(db) == 50 { 150 } else { 120 })),
        style_label: label_of(crate::attributes::style_field(ratings)),
        primary_team_id,
        primary_team_name,
        jersey_number: fv_name(all_fields, "jersey").and_then(|f| f.decoded),
    }
}

/// The `StEO` record index whose `vfEq` equals `arena_id`.
fn arena_record(db: &TdbFile, arena_id: u64) -> Option<u32> {
    let arena = db.table_canonical(ARENA_TABLE)?;
    (0..arena.current_records as u32)
        .find(|&i| read_u64_by_studio(db, arena, i, ARENA_KEY) == Some(arena_id))
}

/// Resolve a team's home arena (`kOtt.buDa → StEO.vfEq`).
fn arena_ref(db: &TdbFile, meta: &FieldMetaIndex, arena_id: u64) -> Option<ArenaRef> {
    let rec = arena_record(db, arena_id)?;
    let arena = db.table_canonical(ARENA_TABLE)?;
    Some(ArenaRef {
        arena_id,
        name: read_str_by_name(db, meta, arena, rec, "arenaname"),
        city: read_str_by_name(db, meta, arena, rec, "cityname"),
    })
}

/// The team master list.
pub fn team_summaries(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    model: &RosterModel,
) -> Vec<TeamSummary> {
    let Some(kott) = db.table_canonical(ids::TEAM) else {
        return Vec::new();
    };
    model
        .teams
        .iter()
        .map(|t| team_summary_at(db, meta, enums, kott, t.team_id, t.record))
        .collect()
}

/// One team summary row (shared by the master list and the arena tenants join).
fn team_summary_at(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    kott: &Table,
    team_id: u64,
    record: u32,
) -> TeamSummary {
    let league = read_u64_by_studio(db, kott, record, "xMjj").map(|v| v as i64);
    // Names may live on kOtt (fat schema) or on a joined stock/custom table (NHL07 thin index).
    let src = team_name_source(db, meta, kott, record);
    TeamSummary {
        team_id,
        record,
        name: read_str_by_name(db, meta, src.table, src.record, "fullname"),
        abbrev: read_str_by_name(db, meta, src.table, src.record, "abbrname"),
        league,
        league_label: league
            .and_then(|v| enums.label("kOtt", "xMjj", v))
            .map(|s| s.to_string()),
    }
}

/// The full Team detail for one `kOtt.vfEq`.
pub fn team_view(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    model: &RosterModel,
    team_id: u64,
) -> Option<TeamView> {
    let team = model.team(team_id)?;
    let kott = db.table_canonical(ids::TEAM)?;
    let all_fields = entity_fields(db, meta, enums, ids::TEAM, team.record);
    let arena = read_u64_by_studio(db, kott, team.record, TEAM_ARENA_FK)
        .and_then(|aid| arena_ref(db, meta, aid));
    // Identity (name/abbrev/…) comes from wherever the names live — kOtt for fat schemas, the joined
    // stock/custom table for NHL07's thin index. `all_fields` stays the raw kOtt row (power-user view).
    let src = team_name_source(db, meta, kott, team.record);
    let identity_fields = if src.studio_id == ids::TEAM {
        all_fields.clone()
    } else {
        entity_fields(db, meta, enums, src.studio_id, src.record)
    };
    // Curated per-game layout: name/classification cells route through the identity source (so
    // NHL07's stock/custom-table names are editable), structural + fat-schema relations stay on
    // kOtt. Venue shows only when the team→arena link exists in this schema.
    let has_arena = all_fields.iter().any(|f| f.name == "arenaid");
    let profile = crate::field_profile::team_field_profile(&crate::field_profile::TeamProfileCtx {
        identity_fields: &identity_fields,
        identity_record: src.record,
        kott_fields: &all_fields,
        kott_record: team.record,
        has_arena,
    });
    let logo_artid = read_u64_by_studio(db, kott, team.record, TEAM_LOGO_ARTID);
    Some(TeamView {
        team_id,
        record: team.record,
        identity: curate(&identity_fields, TEAM_IDENTITY),
        arena,
        roster_count: model.team_roster(team_id).len(),
        retired_numbers: crate::team_tabs::retired_numbers(db, team_id),
        all_fields,
        logo_artid,
        profile,
    })
}

/// The arena master list (iterates the `StEO` table directly).
pub fn arena_summaries(db: &TdbFile, meta: &FieldMetaIndex) -> Vec<ArenaSummary> {
    let Some(arena) = db.table_canonical(ARENA_TABLE) else {
        return Vec::new();
    };
    // Tenant counts: how many kOtt rows point at each arena via buDa.
    let mut tenants: HashMap<u64, u32> = HashMap::new();
    if let Some(kott) = db.table_canonical(ids::TEAM) {
        for rec in 0..kott.current_records as u32 {
            if let Some(aid) = read_u64_by_studio(db, kott, rec, TEAM_ARENA_FK) {
                *tenants.entry(aid).or_insert(0) += 1;
            }
        }
    }
    (0..arena.current_records as u32)
        .filter_map(|i| {
            let arena_id = read_u64_by_studio(db, arena, i, ARENA_KEY)?;
            Some(ArenaSummary {
                arena_id,
                record: i,
                name: read_str_by_name(db, meta, arena, i, "arenaname"),
                city: read_str_by_name(db, meta, arena, i, "cityname"),
                team_count: tenants.get(&arena_id).copied().unwrap_or(0),
            })
        })
        .collect()
}

/// The full Arena detail for one `StEO.vfEq`.
pub fn arena_view(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    arena_id: u64,
) -> Option<ArenaView> {
    let rec = arena_record(db, arena_id)?;
    let all_fields = entity_fields(db, meta, enums, ARENA_TABLE, rec);
    // The tenants: every team whose `kOtt.buDa` points here, resolved to names.
    let teams = db
        .table_canonical(ids::TEAM)
        .map(|kott| {
            (0..kott.current_records as u32)
                .filter(|&trec| read_u64_by_studio(db, kott, trec, TEAM_ARENA_FK) == Some(arena_id))
                .filter_map(|trec| {
                    let team_id = read_u64_by_studio(db, kott, trec, "vfEq")?;
                    Some(team_summary_at(db, meta, enums, kott, team_id, trec))
                })
                .collect()
        })
        .unwrap_or_default();
    Some(ArenaView {
        arena_id,
        record: rec,
        identity: curate(&all_fields, ARENA_IDENTITY),
        teams,
        all_fields,
    })
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
    let mut ratings = companion_record
        .map(|rec| entity_fields(db, meta, enums, companion_table, rec))
        .unwrap_or_default();
    // Player Style is position-split — lock the dropdown to the roles this position may hold.
    if matches!(player.kind, PlayerKind::Skater) {
        crate::attributes::restrict_style_options(&mut ratings, player.position);
    }

    let memberships: Vec<RosterMembership> = model
        .slots
        .iter()
        .filter(|s| s.player_id == Some(player_id))
        .map(|s| RosterMembership {
            team_id: s.team_id,
            team_name: team_name_by_id(db, meta, model, s.team_id),
            jersey: s.jersey,
            captaincy: s.captaincy,
            dressed: s.dressed,
        })
        .collect();

    let header = build_player_header(db, meta, model, &all_fields, &ratings, &memberships);
    let data = build_player_data(db, meta, model, &all_fields);
    let attributes = crate::attributes::build_player_attributes(&ratings, companion_table);

    // Curated per-game layout, built from the loaded bio fields (evidence-first: absent fields
    // never emit a cell). The `dXSB` main-team offset is the probed encoding; the archetype cell
    // shows only when the ratings companion carries the style enum.
    let main_team_offset = probe_main_team_encoding(db, model).offset() as u8;
    let has_style = crate::attributes::style_field(&ratings).is_some();
    let unverified = |label: &str| unverified_note(db, meta, label);
    let profile = crate::field_profile::player_field_profile(
        &all_fields,
        &crate::field_profile::PlayerProfileCtx {
            bio_record: player.bio_record,
            main_team_offset,
            has_style,
            style_label: header.style_label.as_deref(),
            data: &data,
            unverified: &unverified,
        },
    );

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
        header,
        data,
        attributes,
        profile,
    })
}
