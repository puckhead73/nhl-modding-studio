//! `tdb-formula` — the FHM→EA rating formula engine. Given a player's FHM attribute values (+ a
//! [`StatsContext`] for the career-driven prebuilts) and a [`FormulaProfile`], computes the EA
//! ratings, Overall, and archetype/style. Pure and testable; no I/O.
//!
//! Ported from the prior `nhl-database-studio` app under clean-room grant — the full spec (scale,
//! normalisation modes, prebuilts, offset encoding, archetype tables, overall regressions) lives in
//! `docs/reference/fhm-ea-formula.md`, and the default preset is baked in via [`FormulaProfile::default_preset`].

pub mod archetype;
pub mod catalog;
pub mod compute;
pub mod overall;
pub mod prebuilt;
pub mod profile;

use serde::{Deserialize, Serialize};

pub use archetype::Archetype;
pub use compute::compute;
pub use prebuilt::{name_key, StatsContext};
pub use profile::{Component, FormulaProfile, NormalizationMode};

/// The three formula position buckets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionGroup {
    Forward,
    Defenseman,
    Goalie,
}

impl PositionGroup {
    /// The lowercase label the profile/archetype tables key on.
    pub fn as_str(self) -> &'static str {
        match self {
            PositionGroup::Forward => "forward",
            PositionGroup::Defenseman => "defenseman",
            PositionGroup::Goalie => "goalie",
        }
    }

    /// Map a person's primary-position label (`C`/`LW`/`RW`/`D`/`G`) to a formula bucket.
    pub fn from_position_label(label: &str) -> Option<Self> {
        match label {
            "G" => Some(PositionGroup::Goalie),
            "D" | "LD" | "RD" => Some(PositionGroup::Defenseman),
            "C" | "LW" | "RW" | "F" => Some(PositionGroup::Forward),
            _ => None,
        }
    }
}

/// One computed EA rating, on its display scale, tagged with the raw-bits offset for write-back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EaRating {
    pub ea_key: String,
    /// Display value (36..99 standard, 0..15 bias, 0..2 tier); `None` when no component contributed.
    pub value: Option<i32>,
    /// Subtract from `value` to get on-disk raw bits (36 standard / 0 bias-tier; PS4 uses 0 — apply
    /// per `Capabilities` at write time).
    pub raw_offset: i32,
    pub scale_min: i32,
    pub scale_max: i32,
}

/// A player's full computed EA rating set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EaRatingSet {
    /// `forward` / `defenseman` / `goalie`.
    pub position: String,
    pub ratings: Vec<EaRating>,
    pub overall: Option<i32>,
    /// Archetype/style — `None` for goalies (no classifier).
    pub archetype: Option<Archetype>,
}
