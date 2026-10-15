//! Formula orchestration — compute a person-season's EA ratings from the corpus + a profile. Bridges
//! the read-only [`FhmCorpus`] (FHM inputs) and the pure [`tdb_formula`] engine, so `studio-api` sees
//! one call rather than reaching past the domain into either crate.

use std::collections::{BTreeMap, BTreeSet};

use tdb_fhm_store::AttributeMaxima;
use tdb_formula::{
    compute, name_key, EaRating, EaRatingSet, FormulaProfile, PositionGroup, StatsContext,
};

use crate::historical::{FhmCorpus, FhmError};

pub use tdb_formula::{catalog, Archetype};

/// A computed rating set plus the raw FHM inputs it was derived from (for the Attributes tab).
pub struct ComputedRatings {
    pub set: EaRatingSet,
    /// The person-season's raw FHM attribute values (name → value), including position grades.
    pub fhm_values: std::collections::BTreeMap<String, f64>,
}

/// The built-in default formula profile (the packaged preset).
pub fn default_profile() -> FormulaProfile {
    FormulaProfile::default_preset()
}

/// Compute one person-season's EA rating set under `profile`. Returns `None` when the person is
/// absent or has no derivable position (no ratings row anywhere → no formula bucket).
///
/// `maxima` supplies the per-position corpus maxima for `PositionMax` normalization; pass
/// `&AttributeMaxima::default()` for `Nominal` (which ignores it). It is a parameter rather than
/// computed here so the caller can compute/cache the (expensive) scan off the hot path — see
/// [`tdb_fhm_store::compute_maxima_at`].
pub fn compute_person_ratings(
    corpus: &FhmCorpus,
    person_id: i64,
    season_year: i64,
    profile: &FormulaProfile,
    maxima: &AttributeMaxima,
) -> Result<Option<ComputedRatings>, FhmError> {
    let Some(id) = corpus.person_identity(person_id)? else {
        return Ok(None);
    };
    let Some(group) = id
        .position
        .as_deref()
        .and_then(PositionGroup::from_position_label)
    else {
        return Ok(None);
    };

    let fhm_values = corpus.player_fhm_values(person_id, season_year)?;
    let agg = corpus.career_agg(person_id, id.is_goalie)?;
    let stats = StatsContext {
        position: id.position.clone().unwrap_or_default(),
        career_g_pro: agg.career_g,
        career_a_pro: agg.career_a,
        career_gp_pro: agg.career_gp,
        recent_gp: agg.recent_gp,
        recent_goals: agg.recent_goals,
        recent_team_schedule: agg.recent_team_schedule,
        name_key: name_key(&id.first_name, &id.last_name),
    };

    let bucket = match group {
        PositionGroup::Forward => &maxima.forward,
        PositionGroup::Defenseman => &maxima.defenseman,
        PositionGroup::Goalie => &maxima.goalie,
    };

    let set = compute(profile, group, &fhm_values, &stats, bucket);
    Ok(Some(ComputedRatings { set, fhm_values }))
}

/// One playing season's EA Overall, for the season dropdown (newest first).
pub struct CareerSeasonOverall {
    pub season_year: i64,
    pub overall: Option<i32>,
}

/// Cross-career EA rating compilations for the Attributes tab: the per-season overalls (with the
/// best one flagged for a ★), plus the Full Career and Peak-3 averaged views.
pub struct CareerRatings {
    /// `forward` / `defenseman` / `goalie` — the group every view was computed under.
    pub position: String,
    /// Every playing season with its EA Overall, newest first.
    pub seasons: Vec<CareerSeasonOverall>,
    /// Highest-Overall season (ties → earliest) — backs the ★ marker. `None` for goalies / no data.
    pub best_season_year: Option<i64>,
    /// Mean of the per-season computed ratings over ALL playing seasons with data.
    pub career_average: Option<ComputedRatings>,
    /// Same fold over the 3 highest-Overall seasons.
    pub peak3: Option<ComputedRatings>,
}

/// Compute a player's EA ratings for every playing season and fold them into the cross-career views
/// the Attributes tab renders (Full Career + Peak 3) plus the per-season overalls and best season.
///
/// The averages are of the per-season COMPUTED ratings (each season runs the full formula pipeline),
/// not a formula pass over averaged inputs — prebuilt formulas and normalisation are season-relative,
/// so averaging their outputs is the faithful career view. Ported from the prior app's `ratings.rs`.
pub fn compute_person_career_ratings(
    corpus: &FhmCorpus,
    person_id: i64,
    profile: &FormulaProfile,
    maxima: &AttributeMaxima,
) -> Result<Option<CareerRatings>, FhmError> {
    let Some((group, computed)) = computed_seasons(corpus, person_id, profile, maxima)? else {
        return Ok(None);
    };

    let best_season_year = best_season(&computed);
    let seasons = computed
        .iter()
        .map(|s| CareerSeasonOverall {
            season_year: s.year,
            overall: s.overall,
        })
        .collect();
    let position = group.as_str().to_string();
    let career_average = average_view(&computed.iter().collect::<Vec<_>>(), &position);
    let peak3 = average_view(&top_seasons_by_overall(&computed, 3), &position);

    Ok(Some(CareerRatings {
        position,
        seasons,
        best_season_year,
        career_average,
        peak3,
    }))
}

/// Run the full formula pipeline over every one of a person's playing seasons — the shared
/// intermediate behind both the career compilations and the Legends composite. `None` when the
/// person is absent, has no derivable position, or produced no usable season.
fn computed_seasons(
    corpus: &FhmCorpus,
    person_id: i64,
    profile: &FormulaProfile,
    maxima: &AttributeMaxima,
) -> Result<Option<(PositionGroup, Vec<SeasonComputed>)>, FhmError> {
    let Some(id) = corpus.person_identity(person_id)? else {
        return Ok(None);
    };
    let Some(group) = id
        .position
        .as_deref()
        .and_then(PositionGroup::from_position_label)
    else {
        return Ok(None);
    };
    let years = corpus.person_played_years(person_id)?; // newest first
    if years.is_empty() {
        return Ok(None);
    }

    // Career stats context is career-wide (season-independent), so build it once.
    let agg = corpus.career_agg(person_id, id.is_goalie)?;
    let stats = StatsContext {
        position: id.position.clone().unwrap_or_default(),
        career_g_pro: agg.career_g,
        career_a_pro: agg.career_a,
        career_gp_pro: agg.career_gp,
        recent_gp: agg.recent_gp,
        recent_goals: agg.recent_goals,
        recent_team_schedule: agg.recent_team_schedule,
        name_key: name_key(&id.first_name, &id.last_name),
    };
    let bucket = match group {
        PositionGroup::Forward => &maxima.forward,
        PositionGroup::Defenseman => &maxima.defenseman,
        PositionGroup::Goalie => &maxima.goalie,
    };

    let mut computed: Vec<SeasonComputed> = Vec::new();
    for &year in &years {
        let fhm_values = corpus.player_fhm_values(person_id, year)?;
        let set = compute(profile, group, &fhm_values, &stats, bucket);
        // A season contributes only if it produced ratings or carries raw FHM inputs.
        if !set.ratings.iter().any(|r| r.value.is_some()) && fhm_values.is_empty() {
            continue;
        }
        computed.push(SeasonComputed {
            year,
            overall: set.overall,
            set,
            fhm_values,
        });
    }
    Ok((!computed.is_empty()).then_some((group, computed)))
}

/// A player's Legends rating — the peak-composite the Legends builder ranks on.
pub struct LegendsRatings {
    /// `forward` / `defenseman` / `goalie` — the group the composite was computed under.
    pub position: String,
    /// The composited rating set. Every EA attribute is the mean of that attribute's three highest
    /// season values, and `overall` is the EA Overall formula run over those composites.
    pub set: EaRatingSet,
    /// How many playing seasons fed the composite (fewer than 3 means the mean is over what exists).
    pub seasons_used: i64,
    /// The raw FHM inputs, composited the same way — what the Formulas live preview shows beneath
    /// the EA values, and what the archetype classifier ran on.
    pub fhm_values: BTreeMap<String, f64>,
}

/// Compute a person's Legends composite: each EA attribute independently takes the mean of its three
/// highest season values, then the EA Overall formula runs over the composited attributes.
///
/// This is deliberately NOT the Peak-3 view. Peak 3 picks the three best *seasons* and averages
/// everything from them; the Legends composite lets a winger's best Speed and best Slapshot come
/// from different years, and it re-derives Overall from the composite rather than averaging the
/// per-season Overalls — a mean of Overalls is not the Overall of the mean.
pub fn compute_person_legends_ratings(
    corpus: &FhmCorpus,
    person_id: i64,
    profile: &FormulaProfile,
    maxima: &AttributeMaxima,
) -> Result<Option<LegendsRatings>, FhmError> {
    let Some((group, computed)) = computed_seasons(corpus, person_id, profile, maxima)? else {
        return Ok(None);
    };
    let Some((set, fhm_values)) = legends_view(&computed, group) else {
        return Ok(None);
    };
    Ok(Some(LegendsRatings {
        position: group.as_str().to_string(),
        set,
        seasons_used: computed.len() as i64,
        fhm_values,
    }))
}

/// The number of top seasons composited per attribute.
const LEGENDS_TOP_N: usize = 3;

/// Fold per-season computed ratings into the Legends composite (see
/// [`compute_person_legends_ratings`]). `None` for an empty season set.
fn legends_view(
    seasons: &[SeasonComputed],
    group: PositionGroup,
) -> Option<(EaRatingSet, BTreeMap<String, f64>)> {
    let template_set = &seasons.first()?.set;

    // Per EA attribute: the mean of its three highest season values.
    let ratings: Vec<EaRating> = template_set
        .ratings
        .iter()
        .map(|template| EaRating {
            ea_key: template.ea_key.clone(),
            value: top_mean(seasons.iter().filter_map(|s| {
                s.set
                    .ratings
                    .iter()
                    .find(|r| r.ea_key == template.ea_key)
                    .and_then(|r| r.value)
                    .map(|v| v as f64)
            }))
            .map(|m| m.round() as i32),
            raw_offset: template.raw_offset,
            scale_min: template.scale_min,
            scale_max: template.scale_max,
        })
        .collect();

    // Overall from the COMPOSITED attributes — the same lookup shape `compute` uses per season.
    let lookup = |k: &str| {
        ratings
            .iter()
            .find(|r| r.ea_key == k)
            .and_then(|r| r.value)
            .map(|v| v as f64)
    };
    let overall = Some(match group {
        PositionGroup::Goalie => tdb_formula::overall::ea_goalie_overall(lookup),
        PositionGroup::Defenseman => tdb_formula::overall::ea_skater_overall(lookup, true),
        PositionGroup::Forward => tdb_formula::overall::ea_skater_overall(lookup, false),
    });

    // Archetype classifies off raw FHM inputs, so composite those the same way before classifying —
    // a Legends player's style should describe the peak version of them, not their last season.
    let mut composite_fhm: BTreeMap<String, f64> = BTreeMap::new();
    let keys: BTreeSet<&String> = seasons.iter().flat_map(|s| s.fhm_values.keys()).collect();
    for key in keys {
        if let Some(m) = top_mean(
            seasons
                .iter()
                .filter_map(|s| s.fhm_values.get(key).copied()),
        ) {
            composite_fhm.insert(key.clone(), m);
        }
    }

    Some((
        EaRatingSet {
            position: group.as_str().to_string(),
            ratings,
            overall,
            archetype: tdb_formula::archetype::classify(&composite_fhm, group.as_str()),
        },
        composite_fhm,
    ))
}

/// Mean of the [`LEGENDS_TOP_N`] highest values in `vals` (all of them when fewer exist).
fn top_mean(vals: impl Iterator<Item = f64>) -> Option<f64> {
    let mut v: Vec<f64> = vals.collect();
    v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    v.truncate(LEGENDS_TOP_N);
    mean(&v)
}

/// One season's fully-computed ratings — the intermediate the career fold averages over.
struct SeasonComputed {
    year: i64,
    overall: Option<i32>,
    set: EaRatingSet,
    fhm_values: BTreeMap<String, f64>,
}

/// Highest-Overall season; ties break toward the EARLIEST (a "peak year" is the first time a player
/// hit their ceiling). `None` when no season carries an overall.
fn best_season(seasons: &[SeasonComputed]) -> Option<i64> {
    let mut best: Option<(i64, i32)> = None;
    for s in seasons {
        let Some(ovr) = s.overall else { continue };
        best = Some(match best {
            Some((by, bo)) if bo > ovr || (bo == ovr && by <= s.year) => (by, bo),
            _ => (s.year, ovr),
        });
    }
    best.map(|(y, _)| y)
}

/// The `n` highest-Overall seasons, ranked `(overall desc, year asc)` for a deterministic cut.
fn top_seasons_by_overall(seasons: &[SeasonComputed], n: usize) -> Vec<&SeasonComputed> {
    let mut ranked: Vec<&SeasonComputed> = seasons.iter().filter(|s| s.overall.is_some()).collect();
    ranked.sort_by(|a, b| b.overall.cmp(&a.overall).then_with(|| a.year.cmp(&b.year)));
    ranked.truncate(n);
    ranked
}

/// Fold per-season computed ratings into one averaged view: each EA attribute (and the Overall) is
/// the rounded mean of its per-season values (skipping seasons where it was `None`); raw FHM columns
/// average over the seasons carrying each column. `None` for an empty set.
fn average_view(seasons: &[&SeasonComputed], position: &str) -> Option<ComputedRatings> {
    let first = seasons.first()?;
    // EA attribute order is identical across seasons (same position), so take the template from the
    // first and average each key's value across seasons.
    let ratings: Vec<EaRating> = first
        .set
        .ratings
        .iter()
        .map(|template| {
            let vals: Vec<f64> = seasons
                .iter()
                .filter_map(|s| {
                    s.set
                        .ratings
                        .iter()
                        .find(|r| r.ea_key == template.ea_key)
                        .and_then(|r| r.value)
                        .map(|v| v as f64)
                })
                .collect();
            EaRating {
                ea_key: template.ea_key.clone(),
                value: mean(&vals).map(|m| m.round() as i32),
                raw_offset: template.raw_offset,
                scale_min: template.scale_min,
                scale_max: template.scale_max,
            }
        })
        .collect();

    let overalls: Vec<f64> = seasons
        .iter()
        .filter_map(|s| s.overall.map(|v| v as f64))
        .collect();
    let overall = mean(&overalls).map(|m| m.round() as i32);

    // Raw FHM columns: union of keys, mean over the seasons carrying each.
    let mut fhm_values: BTreeMap<String, f64> = BTreeMap::new();
    let keys: BTreeSet<&String> = seasons.iter().flat_map(|s| s.fhm_values.keys()).collect();
    for key in keys {
        let vals: Vec<f64> = seasons
            .iter()
            .filter_map(|s| s.fhm_values.get(key).copied())
            .collect();
        if let Some(m) = mean(&vals) {
            fhm_values.insert(key.clone(), m);
        }
    }

    Some(ComputedRatings {
        set: EaRatingSet {
            position: position.to_string(),
            ratings,
            overall,
            archetype: None,
        },
        fhm_values,
    })
}

fn mean(vals: &[f64]) -> Option<f64> {
    (!vals.is_empty()).then(|| vals.iter().sum::<f64>() / vals.len() as f64)
}

#[cfg(test)]
mod career_tests {
    use super::*;

    fn season(year: i64, overall: Option<i32>, speed: Option<i32>) -> SeasonComputed {
        SeasonComputed {
            year,
            overall,
            set: EaRatingSet {
                position: "forward".into(),
                overall,
                archetype: None,
                ratings: vec![EaRating {
                    ea_key: "Speed".into(),
                    value: speed,
                    raw_offset: 36,
                    scale_min: 36,
                    scale_max: 99,
                }],
            },
            fhm_values: BTreeMap::new(),
        }
    }

    #[test]
    fn best_season_is_highest_overall_then_earliest() {
        let s = vec![
            season(2000, Some(80), Some(80)),
            season(2001, Some(90), Some(90)),
            season(2002, Some(90), Some(70)),
        ];
        assert_eq!(
            best_season(&s),
            Some(2001),
            "tie on 90 breaks to the earlier year"
        );
        assert_eq!(best_season(&[season(1999, None, None)]), None);
    }

    #[test]
    fn top_seasons_and_average_fold() {
        let s = vec![
            season(2000, Some(80), Some(80)),
            season(2001, Some(90), Some(90)),
            season(2002, Some(90), Some(70)),
        ];
        let top2 = top_seasons_by_overall(&s, 2);
        assert_eq!(
            top2.iter().map(|x| x.year).collect::<Vec<_>>(),
            vec![2001, 2002]
        );

        let avg = average_view(&s.iter().collect::<Vec<_>>(), "forward").unwrap();
        assert_eq!(avg.set.overall, Some(87)); // (80+90+90)/3 = 86.67 → 87
        assert_eq!(avg.set.ratings[0].value, Some(80)); // (80+90+70)/3 = 80
    }
}
