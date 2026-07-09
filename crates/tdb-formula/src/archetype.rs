//! Archetype classifier → EA Player Style (`QgFs`). Ported from the prior app's `archetype.rs`
//! (Puckvault parity). Weighted-average scorer over FHM columns; forwards have an enforcer
//! short-circuit; goalies get no style. See `docs/reference/fhm-ea-formula.md` §6.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The winning archetype for a player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archetype {
    /// Puckvault label (e.g. "Sniper").
    pub label: String,
    /// EA Player Style enum label (e.g. "Two-Way Forward").
    pub ea_label: String,
    /// EA Player Style raw code (matches `defs/enums/player-style.toml`).
    pub ea_raw: u32,
    /// Winning weighted-average score (None only for the threshold-gated enforcer).
    pub score: Option<f64>,
}

const ENFORCER_FGT_THRESHOLD: f64 = 17.0;
const ENFORCER_AGG_THRESHOLD: f64 = 17.0;

type Weight = (&'static str, f64);

struct ArchetypeDef {
    label: &'static str,
    ea_label: &'static str,
    ea_raw: u32,
    weights: &'static [Weight],
}

const FORWARD_ARCHETYPES: &[ArchetypeDef] = &[
    ArchetypeDef {
        label: "Sniper",
        ea_label: "Sniper",
        ea_raw: 7,
        weights: &[
            ("Shooting Accuracy", 3.0),
            ("Getting Open", 2.5),
            ("Offensive Read", 1.5),
            ("Speed", 1.0),
            ("Acceleration", 1.0),
        ],
    },
    ArchetypeDef {
        label: "Playmaker",
        ea_label: "Playmaker",
        ea_raw: 6,
        weights: &[
            ("Passing", 3.0),
            ("Puck Handling", 2.5),
            ("Offensive Read", 2.0),
            ("Teamplayer", 1.0),
            ("Getting Open", 1.0),
        ],
    },
    ArchetypeDef {
        label: "Power Forward",
        ea_label: "Power Forward",
        ea_raw: 8,
        weights: &[
            ("Strength", 2.5),
            ("Screening", 2.0),
            ("Shooting Accuracy", 1.5),
            ("Hitting", 1.5),
            ("Stamina", 1.0),
        ],
    },
    ArchetypeDef {
        label: "Two Way Forward",
        ea_label: "Two-Way Forward",
        ea_raw: 9,
        weights: &[
            ("Defensive Read", 2.5),
            ("Offensive Read", 2.0),
            ("Checking", 1.5),
            ("Stickchecking", 1.5),
            ("Faceoffs", 1.0),
        ],
    },
    ArchetypeDef {
        label: "Grinder",
        ea_label: "Grinder",
        ea_raw: 5,
        weights: &[
            ("Checking", 3.0),
            ("Hitting", 2.5),
            ("Stickchecking", 2.0),
            ("Defensive Read", 1.5),
            ("Stamina", 1.0),
        ],
    },
];

const DEFENSE_ARCHETYPES: &[ArchetypeDef] = &[
    ArchetypeDef {
        label: "Offensive Defenseman",
        ea_label: "Offensive Defenseman",
        ea_raw: 2,
        weights: &[
            ("Speed", 2.0),
            ("Acceleration", 2.0),
            ("Passing", 2.0),
            ("Offensive Read", 2.0),
            ("Shooting Accuracy", 1.5),
            ("Puck Handling", 1.0),
        ],
    },
    ArchetypeDef {
        label: "Two Way Defensemen",
        ea_label: "Two-Way Defenseman",
        ea_raw: 4,
        weights: &[
            ("Defensive Read", 2.5),
            ("Offensive Read", 2.5),
            ("Positioning", 2.0),
            ("Passing", 1.5),
            ("Stickchecking", 1.5),
        ],
    },
    ArchetypeDef {
        label: "Defensive Defenseman",
        ea_label: "Defensive Defenseman",
        ea_raw: 1,
        weights: &[
            ("Defensive Read", 2.5),
            ("Positioning", 2.5),
            ("Stickchecking", 2.0),
            ("Checking", 2.0),
            ("Hitting", 1.5),
            ("Shot Blocking", 1.5),
        ],
    },
];

fn enforcer_forward() -> Archetype {
    Archetype {
        label: "Enforcer".to_string(),
        ea_label: "Enforcer Forward".to_string(),
        ea_raw: 10,
        score: None,
    }
}

fn score_archetype(arch: &ArchetypeDef, fhm: &BTreeMap<String, f64>) -> Option<f64> {
    let mut total_v = 0.0_f64;
    let mut total_w = 0.0_f64;
    for (col, w) in arch.weights {
        if let Some(v) = fhm.get(*col) {
            total_v += v * w;
            total_w += w;
        }
    }
    (total_w > 0.0).then_some(total_v / total_w)
}

/// Classify a player's primary archetype from FHM values + position group (`"forward"` /
/// `"defenseman"` / `"goalie"`). Returns `None` for goalies or a row with no scorable attributes.
pub fn classify(fhm: &BTreeMap<String, f64>, position: &str) -> Option<Archetype> {
    let (catalog, enforcer_eligible) = match position {
        "forward" => (FORWARD_ARCHETYPES, true),
        "defenseman" => (DEFENSE_ARCHETYPES, false),
        _ => return None,
    };

    if enforcer_eligible {
        if let (Some(f), Some(a)) = (fhm.get("Fighting"), fhm.get("Aggression")) {
            if *f >= ENFORCER_FGT_THRESHOLD && *a >= ENFORCER_AGG_THRESHOLD {
                return Some(enforcer_forward());
            }
        }
    }

    let mut scored: Vec<Archetype> = catalog
        .iter()
        .map(|a| Archetype {
            label: a.label.to_string(),
            ea_label: a.ea_label.to_string(),
            ea_raw: a.ea_raw,
            score: score_archetype(a, fhm),
        })
        .collect();
    // Sort by score desc, None last; stable keeps declaration order for ties (Python max parity).
    scored.sort_by(|a, b| match (a.score, b.score) {
        (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    });
    scored.into_iter().find(|s| s.score.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs.iter().map(|(k, x)| (k.to_string(), *x)).collect()
    }

    #[test]
    fn sniper_and_enforcer_and_goalie() {
        // Distractor columns populated so the missing-column dropout doesn't inflate a rival
        // archetype that only shares one high column (Puckvault-parity test shape).
        let sniper = classify(
            &v(&[
                ("Shooting Accuracy", 20.0),
                ("Getting Open", 18.0),
                ("Offensive Read", 15.0),
                ("Speed", 16.0),
                ("Acceleration", 14.0),
                ("Passing", 8.0),
                ("Checking", 6.0),
                ("Hitting", 5.0),
                ("Strength", 10.0),
            ]),
            "forward",
        )
        .unwrap();
        assert_eq!(sniper.ea_raw, 7);

        let enf = classify(&v(&[("Fighting", 17.0), ("Aggression", 17.0)]), "forward").unwrap();
        assert_eq!(enf.ea_raw, 10); // >= threshold short-circuits

        assert!(classify(&v(&[]), "goalie").is_none());
    }

    #[test]
    fn defensive_d() {
        let d = classify(
            &v(&[
                ("Defensive Read", 20.0),
                ("Positioning", 19.0),
                ("Stickchecking", 18.0),
                ("Checking", 17.0),
                ("Hitting", 16.0),
                ("Shot Blocking", 15.0),
                // Offensive/Two-Way distractors, low so Defensive D wins on merit.
                ("Speed", 5.0),
                ("Acceleration", 6.0),
                ("Passing", 7.0),
                ("Offensive Read", 8.0),
                ("Shooting Accuracy", 4.0),
                ("Puck Handling", 5.0),
            ]),
            "defenseman",
        )
        .unwrap();
        assert_eq!(d.ea_raw, 1);
    }
}
