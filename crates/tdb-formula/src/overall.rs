//! EA Overall regressions — recovered empirically from EA `default.db` in the prior app. Same linear
//! form; inputs are EA DISPLAY ratings (36..99), keyed by EA attribute name. Ported verbatim.

/// Skater terms: `(ea_key, d_coeff, fwd_weight)`. Defenseman = intercept + Σ coeff·v; forward =
/// scale·(Σ weight·v / divisor) + intercept.
const SKATER_OVERALL_TERMS: &[(&str, f64, f64)] = &[
    ("Speed", 0.023699, 3.0),
    ("Body Checking", 0.084108, 2.0),
    ("Endurance", 0.010734, 1.0),
    ("Puck Control", -0.029, 2.0),
    ("Passing", 0.058153, 5.0),
    ("Slapshot Power", 0.071821, 2.0),
    ("Slapshot Accuracy", 0.073114, 2.0),
    ("Wristshot Power", 0.014244, 3.0),
    ("Wristshot Accuracy", -0.00035, 4.0),
    ("Agility", 0.089079, 3.0),
    ("Strength", 0.243921, 3.0),
    ("Acceleration", 0.048991, 3.0),
    ("Balance", -0.01611, 3.0),
    ("Faceoffs", 0.023246, 1.0),
    ("Durability", -0.00716, 1.0),
    ("Deking", 0.037535, 1.0),
    ("Aggressiveness", -0.00211, 1.0),
    ("Poise", 0.086034, 2.0),
    ("Hand-Eye", -0.02652, 1.0),
    ("Shot Blocking", 0.157127, 1.0),
    ("Offensive Awareness", 0.102197, 7.0),
    ("Defensive Awareness", 0.557359, 9.0),
    ("Discipline", 0.008544, 1.0),
    ("Fighting Skill", -0.0169, 1.0),
    ("Stickchecking", 0.270786, 3.0),
];
const SKATER_OVERALL_D_INTERCEPT: f64 = -74.3907;
const SKATER_OVERALL_FWD_SCALE: f64 = 1.9165;
const SKATER_OVERALL_FWD_DIVISOR: f64 = 65.0;
const SKATER_OVERALL_FWD_INTERCEPT: f64 = -78.725;

/// EA skater Overall from per-attribute DISPLAY values, resolved by `value_for`. Missing → 0.
pub fn ea_skater_overall(value_for: impl Fn(&str) -> Option<f64>, is_defenseman: bool) -> i32 {
    let mut acc = 0.0f64;
    for (key, coeff, weight) in SKATER_OVERALL_TERMS {
        let v = value_for(key).unwrap_or(0.0);
        acc += if is_defenseman { coeff * v } else { weight * v };
    }
    let ovr = if is_defenseman {
        SKATER_OVERALL_D_INTERCEPT + acc
    } else {
        SKATER_OVERALL_FWD_SCALE * (acc / SKATER_OVERALL_FWD_DIVISOR) + SKATER_OVERALL_FWD_INTERCEPT
    };
    (ovr.round() as i32).clamp(1, 99)
}

const GOALIE_OVERALL_TERMS: &[(&str, f64)] = &[
    ("Glove Low", 0.250060),
    ("Glove High", 0.175273),
    ("Stick Low", 0.232873),
    ("Stick High", 0.108138),
    ("Speed", -0.012071),
    ("Agility", 0.068729),
    ("Consistency", 0.099379),
    ("Pokechecking", -0.010799),
    ("Endurance", 0.131290),
    ("Breakaways", 0.089280),
    ("Rebounds", 0.139073),
    ("Shot Recovery", 0.172825),
    ("Poise", 0.086183),
    ("Passing", 0.131022),
    ("Angles", 0.170233),
    ("Puck Playing Frequency", 0.033405),
    ("Aggressiveness", 0.024618),
    ("Durability", 0.043238),
    ("Vision", 0.027384),
];
const GOALIE_OVERALL_INTERCEPT: f64 = -79.5961;

/// EA goalie Overall from per-attribute DISPLAY values. Missing → 0.
pub fn ea_goalie_overall(value_for: impl Fn(&str) -> Option<f64>) -> i32 {
    let mut acc = 0.0f64;
    for (key, coeff) in GOALIE_OVERALL_TERMS {
        acc += coeff * value_for(key).unwrap_or(0.0);
    }
    ((GOALIE_OVERALL_INTERCEPT + acc).round() as i32).clamp(1, 99)
}
