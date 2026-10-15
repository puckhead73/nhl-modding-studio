//! Player-row **encoding contract** — the per-title meaning of the bio and companion cells whose
//! metadata is identical across titles but whose stored numbers are not: the weight base, the
//! six-bit rating scale, the birth month / day index and the contract-dollar unit.
//!
//! **Resolved from the schema shape and the declared title, never from cell values.** Until
//! 25 Sep 2026 the weight base and the rating scale were re-derived on every read from the
//! current cell population (a median of the stored weights; the share of rating cells above 49).
//! That reads stock rosters correctly but cannot identify the encoding of an edited file: a roster
//! of 170 lb players on a base-120 title stores raw 50 everywhere and the median rule would call
//! it base 150; a roster whose ratings were all lowered below 50 on a 36-scale title would be
//! re-read on the 50 scale. The stored format did not change — only the values did — and the
//! same values fed the write path, so a second edit would have been written with the wrong offset
//! (development audit 2026-09-25, F1).
//!
//! The contract here keys on the **schema family** — features of the table / field layout that a
//! cell edit cannot move (which draft-year field the bio table carries and how wide it is; the
//! equipment table's widths; the byte order; the gen6 style field) — and, when the project was
//! opened with a title, cross-checks that title against the family. A file whose title and schema
//! disagree is a **conflict**: display stays on the schema contract and the encoded fields refuse
//! to write until the mismatch is resolved. A schema no family claims is **unresolved**: the
//! affected cells show their raw stored value, say so, and are not editable.
//!
//! Every value carries its evidence level. `InGame` was read on a console by the earlier
//! program; `Measured` was pinned against independent known values on the committed anchors;
//! `Inferred` follows from a population argument only (the NHL 08–10 "+50" rating scale).
//!
//! The old population probes remain in the crate as **diagnostics** ([`content_diagnostics`]):
//! they report when a file's cells disagree with its contract — useful to spot a file written by a
//! build that carried the wrong offset — but they never choose the contract.

use tdb_core::{Endian, Table, TdbFile};
use tdb_defs::FieldMetaIndex;
use tdb_relations::entity::ids;

use crate::assets::BirthdateEncoding;
use crate::project::Platform;

/// The layout families the committed anchors fall into, by schema shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaFamily {
    /// NHL 07 (gen6 layout; Xbox 360 shipping target, PS3 research copy).
    Gen6Nhl07,
    /// NHL 08 (Xbox 360 / PS3): 7-bit `yeardrafted`.
    Gen7Nhl08,
    /// NHL 09 (Xbox 360 / PS3): 8-bit `yeardrafted`, 4-bit `helmetbrand`.
    Gen7Nhl09,
    /// NHL 10 (Xbox 360 / PS3): 8-bit `yeardrafted`, 8-bit `helmetbrand`.
    Gen7Nhl10,
    /// NHL 11 → NHL 15 and NHL Legacy (Xbox 360 / PS3; Legacy Recompiled by identity): `draftyear`.
    Gen7Nhl11ToLegacy,
    /// PS4 NHL 15 → NHL 21 (little-endian `nhl_game.db`).
    Ps4,
}

impl SchemaFamily {
    /// Human-readable family name for notes and refusals.
    pub fn label(self) -> &'static str {
        match self {
            SchemaFamily::Gen6Nhl07 => "NHL 07",
            SchemaFamily::Gen7Nhl08 => "NHL 08",
            SchemaFamily::Gen7Nhl09 => "NHL 09",
            SchemaFamily::Gen7Nhl10 => "NHL 10",
            SchemaFamily::Gen7Nhl11ToLegacy => "NHL 11 – NHL Legacy",
            SchemaFamily::Ps4 => "PS4 NHL 15 – 21",
        }
    }

    /// The family a studio game id on a platform is expected to carry. `None` for an unknown id.
    pub fn of_game(game_id: &str, platform: Platform) -> Option<SchemaFamily> {
        if platform == Platform::Ps4 {
            return matches!(
                game_id,
                "nhl15" | "nhl16" | "nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21"
            )
            .then_some(SchemaFamily::Ps4);
        }
        Some(match game_id {
            "nhl07" => SchemaFamily::Gen6Nhl07,
            "nhl08" => SchemaFamily::Gen7Nhl08,
            "nhl09" => SchemaFamily::Gen7Nhl09,
            "nhl10" => SchemaFamily::Gen7Nhl10,
            "nhl11" | "nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy" | "nhllegacy-recompiled" | "legacyrecomp" => {
                SchemaFamily::Gen7Nhl11ToLegacy
            }
            _ => return None,
        })
    }
}

fn field_named<'a>(
    meta: &FieldMetaIndex,
    table: &'a Table,
    name: &str,
) -> Option<&'a tdb_core::FieldDescriptor> {
    table.fields.iter().find(|f| {
        meta.field(&table.id.raw(), &f.id.raw())
            .is_some_and(|m| m.name == name)
    })
}

/// The descriptor of one encoded field as the metadata declares it: `(bit width, low, high)`.
fn int_field(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    table: &Table,
    studio_id: &str,
    name: &str,
) -> Option<(u32, i64, i64)> {
    let f = table.fields.iter().find(|f| f.id.canonical(db.endian) == studio_id)?;
    let m = meta.field(&table.id.raw(), &f.id.raw())?;
    if m.name != name || !matches!(m.kind, tdb_defs::FieldType::Integer) {
        return None;
    }
    Some((f.bit_width, m.range_low?, m.range_high?))
}

/// `(six-bit 0..63 ratings, seven-bit 0..99 ratings)` on a companion table, by metadata class;
/// `None` when a rating's width contradicts its declared range.
fn rating_classes(db: &TdbFile, meta: &FieldMetaIndex, studio_table: &str) -> Option<(usize, usize)> {
    let t = db.table_canonical(studio_table)?;
    let (mut six, mut ninety_nine) = (0, 0);
    for f in &t.fields {
        let Some(m) = meta.field(&t.id.raw(), &f.id.raw()) else {
            continue;
        };
        match crate::attributes::rating_offset(studio_table, &m.name, m.range_low, m.range_high) {
            Some(36) if f.bit_width == 6 => six += 1,
            Some(0) if f.bit_width == 7 => ninety_nine += 1,
            Some(_) => return None,
            None => {}
        }
    }
    Some((six, ninety_nine))
}

/// Classify a database by a bounded **layout signature** of the fields the contract encodes
/// (review 2026-09-26 F2). Nothing here reads a cell value, and nothing requires stock bytes:
/// capacities, record counts and unrelated tables may differ. A family is named only when every
/// discriminator agrees with its measured signature (`f2b-layout-descriptor-measurement.log`,
/// 27 anchor directories):
///
/// | Family | order | `qFnd` year | `contractdollars` | draft field | `helmetbrand` | ratings |
/// |---|---|---|---|---|---|---|
/// | NHL 07 | big | 6 b | 9 b | `yeardrafted` 6 b | (no equipment table) | 6 b 0..63 |
/// | NHL 08 | big | 7 b | 10 b | `yeardrafted` 7 b | 4 b | 6 b 0..63 |
/// | NHL 09 | big | 8 b | 10 b | `yeardrafted` 8 b | 4 b | 6 b 0..63 |
/// | NHL 10 | big | 8 b | 10 b | `yeardrafted` 8 b | 8 b | 6 b 0..63 |
/// | NHL 11 – Legacy | big | 8 b | 13 b | `draftyear` 8 b | 8 b | 6 b 0..63 |
/// | PS4 15 – 21 | little | 8 b | 13 / 14 b | `draftyear` 8 b | 8 / 9 / 10 b | 7 b 0..99 |
///
/// Shared by every family: `ubPc.sNZW` "weight" 8 b 0..255, `JKLp` "month" 4 b 0..15 (0..11 on
/// PS4 16–21), `Kswi` "day" 5 b 0..31 (0..30 on PS4 16–21), integer typed. Anything else — a
/// little-endian file with six-bit ratings, a missing or re-typed encoded field, an unmeasured
/// width — is **unresolved**: the encoded cells stay raw and read-only.
pub fn schema_family(db: &TdbFile, meta: &FieldMetaIndex) -> Option<SchemaFamily> {
    let bio = db.table_canonical(ids::BIO)?;
    if int_field(db, meta, bio, "sNZW", "weight")? != (8, 0, 255) {
        return None;
    }
    let month = int_field(db, meta, bio, "JKLp", "month")?;
    let day = int_field(db, meta, bio, "Kswi", "day")?;
    let short_calendar = match (month, day) {
        ((4, 0, 15), (5, 0, 31)) => false,
        ((4, 0, 11), (5, 0, 30)) => true,
        _ => return None,
    };
    let (year_w, ..) = int_field(db, meta, bio, "qFnd", "year")?;
    let cap_w = field_named(meta, bio, "contractdollars")?.bit_width;
    let (sk6, sk99) = rating_classes(db, meta, ids::SKATER_AI)?;
    let (gl6, gl99) = rating_classes(db, meta, ids::GOALIE_AI)?;
    let six_bit = sk6 > 0 && gl6 > 0 && sk99 == 0 && gl99 == 0;
    let ninety_nine = sk99 > 0 && gl99 > 0 && sk6 == 0 && gl6 == 0;
    let draftyear = field_named(meta, bio, "draftyear").map(|f| f.bit_width);
    let drafted = field_named(meta, bio, "yeardrafted").map(|f| f.bit_width);
    let helmet = db
        .table_canonical(ids::SKATER_EQUIP)
        .and_then(|t| field_named(meta, t, "helmetbrand"))
        .map(|f| f.bit_width);

    if db.endian == Endian::Little {
        let ok = ninety_nine
            && draftyear == Some(8)
            && drafted.is_none()
            && year_w == 8
            && matches!(cap_w, 13 | 14)
            && matches!(helmet, Some(8..=10));
        return ok.then_some(SchemaFamily::Ps4);
    }
    if !six_bit || short_calendar {
        return None;
    }
    match (draftyear, drafted, year_w, cap_w, helmet) {
        (Some(8), None, 8, 13, Some(8)) => Some(SchemaFamily::Gen7Nhl11ToLegacy),
        (None, Some(6), 6, 9, None) => Some(SchemaFamily::Gen6Nhl07),
        (None, Some(7), 7, 10, Some(4)) => Some(SchemaFamily::Gen7Nhl08),
        (None, Some(8), 8, 10, Some(4)) => Some(SchemaFamily::Gen7Nhl09),
        (None, Some(8), 8, 10, Some(8)) => Some(SchemaFamily::Gen7Nhl10),
        _ => None,
    }
}

/// How a contract value was established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// Read on a console by the earlier verification program.
    InGame,
    /// Pinned against independent known values on the committed anchors.
    Measured,
    /// A population argument only — provisional until a console reading closes it.
    Inferred,
}

impl Evidence {
    pub fn label(self) -> &'static str {
        match self {
            Evidence::InGame => "in-game verified",
            Evidence::Measured => "measured against known values",
            Evidence::Inferred => "inferred from the stock distribution (provisional)",
        }
    }
}

/// The resolved per-title meaning of the encoded player cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerEncoding {
    pub family: SchemaFamily,
    /// `ubPc.sNZW` stores pounds minus this (0 = unset).
    pub weight_base: i64,
    pub weight_evidence: Evidence,
    /// The display / edit offset of the six-bit (0..63) companion ratings. Families without
    /// six-bit ratings (PS4 stores 0..99 in seven bits) never consult it.
    pub sixbit_rating_offset: i64,
    pub rating_evidence: Evidence,
    /// How `ubPc` stores the birth month and day.
    pub birthdate: BirthdateEncoding,
    pub birthdate_evidence: Evidence,
    /// Dollars per stored unit of `contractdollars`, when witnessed for the family.
    pub cap_hit_unit: Option<u64>,
    pub cap_hit_evidence: Option<Evidence>,
}

/// The contract for a family. Evidence, per value (25 Sep 2026):
/// * weight base — Kukkonen 190 lb on every title that lists him (raw 40 on 07–10, 70 after),
///   `tests/weight_base.rs`; the NHL 07 value was also read in game by the earlier program.
/// * rating offset — NHL 07 +50 in game; NHL 08–10 +50 inferred (no stock rating above 49 —
///   `tests/rating_scale_probe.rs`); NHL 14 / 15 / Legacy +36 in game, NHL 11–13 by the same
///   layout family; PS4 stores the human value (metadata 0..99).
/// * birthdate — measured on every anchor (`tests/birthdate_encoding.rs`, receipt 11).
/// * cap-hit unit — NHL 07 ×$25 000 in game; NHL 08 / 09 / 10 ×$25 000 and PS4 ×$5 000 measured
///   against several public cap hits per family (`tests/contract_scale_probe.rs`, candidate table:
///   NHL 08 Iginla / Lidstrom / Luongo / Pronger / Richards / Thornton / Kovalchuk — Crosby's NHL 08
///   figure is EA's own and does not match his cap hit); NHL 11–Legacy ×$5 000 by the earlier
///   program and the same candidates. A family none of the candidates witnesses would carry `None`
///   and show its cell raw.
pub fn encoding_for_family(family: SchemaFamily) -> PlayerEncoding {
    use Evidence::*;
    use SchemaFamily::*;
    let (weight_base, weight_evidence) = match family {
        Gen6Nhl07 => (150, InGame),
        Gen7Nhl08 | Gen7Nhl09 | Gen7Nhl10 => (150, Measured),
        Gen7Nhl11ToLegacy | Ps4 => (120, Measured),
    };
    let (sixbit_rating_offset, rating_evidence) = match family {
        Gen6Nhl07 => (50, InGame),
        Gen7Nhl08 | Gen7Nhl09 | Gen7Nhl10 => (50, Inferred),
        Gen7Nhl11ToLegacy => (36, InGame),
        Ps4 => (0, Measured),
    };
    let birthdate = match family {
        Gen6Nhl07 | Gen7Nhl08 | Gen7Nhl09 | Gen7Nhl10 => BirthdateEncoding::OneIndexed,
        Gen7Nhl11ToLegacy | Ps4 => BirthdateEncoding::ZeroIndexed,
    };
    let (cap_hit_unit, cap_hit_evidence) = match family {
        Gen6Nhl07 => (Some(25_000), Some(InGame)),
        Gen7Nhl08 | Gen7Nhl09 | Gen7Nhl10 => (Some(25_000), Some(Measured)),
        Gen7Nhl11ToLegacy => (Some(5_000), Some(InGame)),
        Ps4 => (Some(5_000), Some(Measured)),
    };
    PlayerEncoding {
        family,
        weight_base,
        weight_evidence,
        sixbit_rating_offset,
        rating_evidence,
        birthdate,
        birthdate_evidence: Measured,
        cap_hit_unit,
        cap_hit_evidence,
    }
}

/// The contract for a database, from its schema shape. `None` when no family claims the schema —
/// callers show the affected cells raw and refuse to encode them.
pub fn player_encoding(db: &TdbFile, meta: &FieldMetaIndex) -> Option<PlayerEncoding> {
    schema_family(db, meta).map(encoding_for_family)
}

/// The text shown for an encoded cell whose schema is unresolved.
pub const UNRESOLVED_NOTE: &str = "raw stored value — this schema matches no known title family, so its encoding is unresolved and the cell is not editable";

/// When the project was opened as a title whose family differs from the schema's, the reason the
/// encoded cells refuse to write. `None` when no title is known or the two agree.
pub fn encoding_conflict(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    game_id: Option<&str>,
    platform: Platform,
) -> Option<String> {
    let game = game_id?;
    let Some(declared) = SchemaFamily::of_game(game, platform) else {
        return Some(format!(
            "this project was opened as {game} on {}, which is not a title / platform combination with a known player-row encoding. Weight, ratings, birth dates and contract dollars are not editable until the project is reopened as a supported title",
            platform_label(platform)
        ));
    };
    let schema = schema_family(db, meta)?;
    (declared != schema).then(|| {
        format!(
            "this project was opened as {} on {}, whose player rows are encoded as {}, but the database's schema is the {} layout. Weight, ratings, birth dates and contract dollars are shown on the schema's encoding and are not editable until the project is reopened as the matching title",
            game,
            platform_label(platform),
            declared.label(),
            schema.label()
        )
    })
}

fn platform_label(p: Platform) -> &'static str {
    match p {
        Platform::Ps3 => "PS3",
        Platform::Xbox => "Xbox 360",
        Platform::Ps4 => "PS4",
        Platform::Recomp => "Legacy Recompiled",
    }
}

/// Whether a `(studio table, studio field)` cell is one the contract encodes — the cells a
/// conflict or an unresolved schema makes read-only.
pub fn is_contract_encoded(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    studio_table: &str,
    studio_field: &str,
) -> bool {
    if studio_table == ids::BIO {
        if matches!(studio_field, "sNZW" | "JKLp" | "Kswi") {
            return true;
        }
        return db
            .table_canonical(ids::BIO)
            .and_then(|bio| field_named(meta, bio, "contractdollars"))
            .is_some_and(|f| f.id.canonical(db.endian) == studio_field);
    }
    if studio_table == ids::SKATER_AI || studio_table == ids::GOALIE_AI {
        let Some(t) = db.table_canonical(studio_table) else {
            return false;
        };
        let Some(f) = t
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == studio_field)
        else {
            return false;
        };
        return f.bit_width == 6
            && meta.field(&t.id.raw(), &f.id.raw()).is_some_and(|m| {
                crate::attributes::rating_offset(studio_table, &m.name, m.range_low, m.range_high)
                    == Some(36)
            });
    }
    false
}

/// **Diagnostics only.** The population probes the contract replaced, compared to the contract:
/// one line per disagreement, empty when the cells look like the title's stock data. A
/// disagreement is a hint that the file was edited by a build carrying the wrong offset (or that
/// the roster is unusual), never a reason to change the contract.
pub fn content_diagnostics(db: &TdbFile, meta: &FieldMetaIndex) -> Vec<String> {
    let mut out = Vec::new();
    let Some(enc) = player_encoding(db, meta) else {
        return vec![UNRESOLVED_NOTE.to_string()];
    };
    if let Some(bio) = db.table_canonical(ids::BIO) {
        let probed = crate::assets::probe_weight_base(db, bio);
        if probed != enc.weight_base {
            out.push(format!(
                "weight: the stored weights have the shape of a base-{probed} population, but the {} contract is base {} — check whether this file was edited by a build before 25 Sep 2026",
                enc.family.label(),
                enc.weight_base
            ));
        }
        let probed_bd = crate::assets::probe_birthdate_encoding(db, bio);
        if probed_bd != enc.birthdate {
            out.push(format!(
                "birth dates: the stored months / days look {:?}, the {} contract is {:?}",
                probed_bd,
                enc.family.label(),
                enc.birthdate
            ));
        }
    }
    if enc.family != SchemaFamily::Ps4 {
        let probed = crate::view::sixbit_rating_population_offset(db, meta);
        if probed != enc.sixbit_rating_offset {
            out.push(format!(
                "ratings: the six-bit rating cells have the shape of a +{probed} population, but the {} contract is +{}",
                enc.family.label(),
                enc.sixbit_rating_offset
            ));
        }
    }
    out
}
