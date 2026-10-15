//! The core compute: one EA attribute value from its components (weighted average of normalised FHM
//! attrs + prebuilts), and the full [`EaRatingSet`] for a player. Ported from the prior app's
//! `compute_ea_value` (see `docs/reference/fhm-ea-formula.md` §3).

use std::collections::BTreeMap;

use crate::catalog::{ea_attr_scale, prebuilt_lookup, GOALIE_EA_ATTRS, SKATER_EA_ATTRS};
use crate::prebuilt::{eval_prebuilt, StatsContext};
use crate::profile::{FormulaProfile, NormalizationMode};
use crate::{EaRating, EaRatingSet, PositionGroup};

/// FHM's nominal cap under `Nominal` mode.
const NOMINAL_FHM_MAX: f64 = 20.0;

/// Compute one EA attribute's value. `None` when no component contributed (row shows "—").
pub(crate) fn compute_ea_value(
    components: &[(String, f64)],
    fhm_values: &BTreeMap<String, f64>,
    position_maxes: &BTreeMap<String, f64>,
    stats: &StatsContext,
    scale: (i32, i32),
    mode: NormalizationMode,
) -> Option<i32> {
    if components.is_empty() {
        return None;
    }
    let (lo, hi) = (scale.0 as f64, scale.1 as f64);
    if hi <= lo {
        return None;
    }

    // Non-standard bias/tier rows: delegate to the first prebuilt component, verbatim on its scale.
    let is_standard = (lo - 36.0).abs() < f64::EPSILON && (hi - 99.0).abs() < f64::EPSILON;
    if !is_standard {
        for (key, _w) in components {
            if let Some((bare, _n, _d)) = prebuilt_lookup(key) {
                if let Some(val) = eval_prebuilt(bare, stats, fhm_values) {
                    return Some(val.round().clamp(lo, hi) as i32);
                }
            }
        }
        return None;
    }

    let mut total_w = 0.0;
    let mut total_v = 0.0;
    for (key, weight) in components {
        if let Some((bare, native_max, _d)) = prebuilt_lookup(key) {
            let Some(val) = eval_prebuilt(bare, stats, fhm_values) else {
                continue;
            };
            let native_max = native_max as f64;
            if native_max <= 0.0 {
                continue;
            }
            total_v += (val / native_max) * weight;
            total_w += weight.abs();
            continue;
        }
        // Raw FHM attribute; denominator depends on the mode.
        let Some(val) = fhm_values.get(key) else {
            continue;
        };
        let denom = match mode {
            NormalizationMode::Nominal => NOMINAL_FHM_MAX,
            NormalizationMode::PositionMax => position_maxes
                .get(key)
                .copied()
                .filter(|m| *m > 0.0)
                .unwrap_or(NOMINAL_FHM_MAX),
        };
        if denom <= 0.0 {
            continue;
        }
        total_v += (val / denom) * weight;
        total_w += weight.abs();
    }
    if total_w <= 0.0 {
        return None;
    }
    let raw = lo + (total_v / total_w) * (hi - lo);
    Some(raw.clamp(lo, hi).round() as i32)
}

/// Compute the full EA rating set for a player: every EA attribute for the position, plus overall
/// and (skaters only) archetype.
pub fn compute(
    profile: &FormulaProfile,
    group: PositionGroup,
    fhm_values: &BTreeMap<String, f64>,
    stats: &StatsContext,
    position_maxes: &BTreeMap<String, f64>,
) -> EaRatingSet {
    let pos = group.as_str();
    let attrs: &[&str] = if group == PositionGroup::Goalie {
        GOALIE_EA_ATTRS
    } else {
        SKATER_EA_ATTRS
    };

    let ratings: Vec<EaRating> = attrs
        .iter()
        .map(|&ea_key| {
            let scale = ea_attr_scale(ea_key);
            let value = compute_ea_value(
                profile.components(pos, ea_key),
                fhm_values,
                position_maxes,
                stats,
                scale,
                profile.normalisation_mode,
            );
            EaRating {
                ea_key: ea_key.to_string(),
                value,
                raw_offset: crate::catalog::ea_attr_raw_offset(ea_key),
                scale_min: scale.0,
                scale_max: scale.1,
            }
        })
        .collect();

    let lookup = |k: &str| {
        ratings
            .iter()
            .find(|r| r.ea_key == k)
            .and_then(|r| r.value)
            .map(|v| v as f64)
    };
    let overall = Some(match group {
        PositionGroup::Goalie => crate::overall::ea_goalie_overall(lookup),
        PositionGroup::Defenseman => crate::overall::ea_skater_overall(lookup, true),
        PositionGroup::Forward => crate::overall::ea_skater_overall(lookup, false),
    });

    let archetype = crate::archetype::classify(fhm_values, pos);

    EaRatingSet {
        position: pos.to_string(),
        ratings,
        overall,
        archetype,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fhm(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn nominal_maps_fhm_20_to_ea_99_and_0_to_36() {
        let p = FormulaProfile::default_preset();
        let maxes = BTreeMap::new();
        // Speed = FHM Speed ×1 / 20 → 36 + x/20×63.
        let top = fhm(&[("Speed", 20.0)]);
        let set = compute(
            &p,
            PositionGroup::Forward,
            &top,
            &StatsContext::default(),
            &maxes,
        );
        let speed = set.ratings.iter().find(|r| r.ea_key == "Speed").unwrap();
        assert_eq!(speed.value, Some(99));
        assert_eq!(speed.raw_offset, 36);

        let bottom = fhm(&[("Speed", 0.0)]);
        let set = compute(
            &p,
            PositionGroup::Forward,
            &bottom,
            &StatsContext::default(),
            &maxes,
        );
        assert_eq!(
            set.ratings
                .iter()
                .find(|r| r.ea_key == "Speed")
                .unwrap()
                .value,
            Some(36)
        );
    }

    #[test]
    fn bias_row_is_direct_output_on_native_scale() {
        let p = FormulaProfile::default_preset();
        let maxes = BTreeMap::new();
        // No career stats → pass_shoot_bias sits at neutral 8, def_off_bias at 8.
        let set = compute(
            &p,
            PositionGroup::Forward,
            &fhm(&[]),
            &StatsContext::default(),
            &maxes,
        );
        let bias = set
            .ratings
            .iter()
            .find(|r| r.ea_key == "Shoot/Pass Bias")
            .unwrap();
        assert_eq!(bias.value, Some(8));
        assert_eq!(bias.scale_min, 0);
        assert_eq!(bias.scale_max, 15);
        assert_eq!(bias.raw_offset, 0);
    }

    #[test]
    fn overall_and_archetype_populate() {
        let p = FormulaProfile::default_preset();
        let maxes = BTreeMap::new();
        let sniper = fhm(&[
            ("Shooting Accuracy", 19.0),
            ("Getting Open", 18.0),
            ("Offensive Read", 17.0),
            ("Speed", 18.0),
            ("Acceleration", 17.0),
            ("Shooting", 18.0),
            ("Skating", 18.0),
            // distractors so the single-column dropout doesn't hand it to Power Forward
            ("Passing", 8.0),
            ("Checking", 6.0),
            ("Hitting", 5.0),
            ("Strength", 10.0),
        ]);
        let set = compute(
            &p,
            PositionGroup::Forward,
            &sniper,
            &StatsContext::default(),
            &maxes,
        );
        // Overall is produced (sparse FHM input → a low but valid 1..99 value).
        assert!(set.overall.is_some());
        assert_eq!(set.archetype.unwrap().ea_raw, 7); // Sniper
    }
}
