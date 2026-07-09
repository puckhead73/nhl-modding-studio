//! Static catalog — the EA attribute lists, the prebuilt-formula registry, and the per-attribute
//! output scale / raw-bits offset. Ported from the prior app's `ratings.rs` (see
//! `docs/reference/fhm-ea-formula.md`).

/// Marks a component key as a prebuilt formula rather than a raw FHM column.
pub const PREBUILT_PREFIX: &str = "prebuilt:";

/// EA skater rating attributes, in display order.
pub const SKATER_EA_ATTRS: &[&str] = &[
    "Speed", "Body Checking", "Endurance", "Puck Control", "Passing", "Shoot/Pass Bias",
    "Defense/Offense Bias", "Pass/Carry Bias", "Slapshot Power", "Slapshot Accuracy",
    "Wristshot Power", "Wristshot Accuracy", "Agility", "Strength", "Acceleration", "Balance",
    "Faceoffs", "Durability", "Deking", "Aggressiveness", "Poise", "Hand-Eye", "Shot Blocking",
    "Offensive Awareness", "Defensive Awareness", "Discipline", "Fighting Skill", "Stickchecking",
];

/// EA goalie rating attributes, in display order.
pub const GOALIE_EA_ATTRS: &[&str] = &[
    "Glove Low", "Glove High", "Stick Low", "Stick High", "Speed", "Agility", "Consistency",
    "Pokechecking", "Endurance", "Breakaways", "Rebounds", "Shot Recovery", "Poise", "Passing",
    "Angles", "Puck Playing Frequency", "Aggressiveness", "Durability", "Vision",
];

/// Prebuilt formulas: `(bare_key, native_max, direct_output)`. `direct_output` rows override the EA
/// scale to their native range and pass through verbatim; others normalise against `native_max`.
pub const PREBUILT_CATALOG: &[(&str, u32, bool)] = &[
    ("pass_shoot_bias", 15, true),
    ("pass_shoot_bias_def", 15, true),
    ("def_off_bias", 15, true),
    ("pass_carry_bias", 15, true),
    ("vezina_consistency", 2, true),
    ("gp_durability", 20, false),
    ("shooting_pct", 30, false),
];

/// Look up a stored `"prebuilt:<key>"` component → `(bare_key, native_max, direct_output)`.
pub fn prebuilt_lookup(stored_key: &str) -> Option<(&'static str, u32, bool)> {
    let bare = stored_key.strip_prefix(PREBUILT_PREFIX)?;
    PREBUILT_CATALOG
        .iter()
        .find(|(k, _, _)| *k == bare)
        .map(|(k, n, d)| (*k, *n, *d))
}

/// Non-standard output scale for a bias/tier EA attribute, as `(lo, hi)`.
pub fn override_scale_for_ea_attr(ea_attr: &str) -> Option<(i32, i32)> {
    match ea_attr {
        "Shoot/Pass Bias" | "Defense/Offense Bias" | "Pass/Carry Bias" => Some((0, 15)),
        "Consistency" => Some((0, 2)),
        _ => None,
    }
}

/// The output display scale for an EA attribute — `(36, 99)` standard, or the bias/tier override.
pub fn ea_attr_scale(ea_attr: &str) -> (i32, i32) {
    override_scale_for_ea_attr(ea_attr).unwrap_or((36, 99))
}

/// The value to SUBTRACT from the display value to get the on-disk raw bits: 36 for standard 6-bit
/// ratings, 0 for the bias/tier rows (stored raw). NOTE: PS4 stores 0..99 directly (offset 0) —
/// callers writing PS4 databases must apply that per `Capabilities`.
pub fn ea_attr_raw_offset(ea_attr: &str) -> i32 {
    override_scale_for_ea_attr(ea_attr).map(|(lo, _)| lo).unwrap_or(36)
}

/// FHM attribute vocabulary for the Formulas UI, grouped: `(group, [(display_label, fhm_key)])`. The
/// keys match the corpus `attrs_json` column names. Ported from the prior app.
pub const FHM_ATTR_GROUPS: &[(&str, &[(&str, &str)])] = &[
    (
        "Character / Mental",
        &[
            ("Aggression", "Aggression"),
            ("Bravery", "Bravery"),
            ("Determination", "Determination"),
            ("Team Player", "Teamplayer"),
            ("Leadership", "Leadership"),
            ("Temperament", "Temperament"),
            ("Professionalism", "Professionalism"),
            ("Mental Toughness", "Mental Toughness"),
        ],
    ),
    (
        "Physical",
        &[
            ("Acceleration", "Acceleration"),
            ("Agility", "Agility"),
            ("Balance", "Balance"),
            ("Speed", "Speed"),
            ("Stamina", "Stamina"),
            ("Strength", "Strength"),
            ("Fighting", "Fighting"),
        ],
    ),
    (
        "Offensive",
        &[
            ("Screening", "Screening"),
            ("Getting Open", "Getting Open"),
            ("Passing", "Passing"),
            ("Puck Handling", "Puck Handling"),
            ("Shooting Acc.", "Shooting Accuracy"),
            ("Shooting Range", "Shooting Range"),
            ("Offensive Read", "Offensive Read"),
        ],
    ),
    (
        "Defensive",
        &[
            ("Checking", "Checking"),
            ("Faceoffs", "Faceoffs"),
            ("Hitting", "Hitting"),
            ("Positioning", "Positioning"),
            ("Shot Blocking", "Shot Blocking"),
            ("Stickchecking", "Stickchecking"),
            ("Defensive Read", "Defensive Read"),
        ],
    ),
    (
        "Goalie",
        &[
            ("Goalie Stamina", "Goalie Stamina"),
            ("Goalie Positioning", "G Positioning"),
            ("Goalie Passing", "G Passing"),
            ("Goalie Pokecheck", "G Pokecheck"),
            ("Blocker", "Blocker"),
            ("Glove", "Glove"),
            ("Rebound", "Rebound"),
            ("Recovery", "Recovery"),
            ("Goalie Puck Handling", "G Puckhandling"),
            ("Low Shots", "Low Shots"),
            ("Goalie Skating", "G Skating"),
            ("Reflexes", "Reflexes"),
        ],
    ),
    (
        "FHM Composites",
        &[
            ("Skating", "Skating"),
            ("Shooting", "Shooting"),
            ("Playmaking", "Playmaking"),
            ("Defending", "Defending"),
            ("Physicality", "Physicality"),
            ("Conditioning", "Conditioning"),
            ("Character", "Character"),
            ("Hockey Sense", "Hockey Sense"),
            ("Goalie Technique", "Goalie Technique"),
            ("Goalie Overall", "Goalie Overall Positioning"),
        ],
    ),
];

/// Prebuilt formula display metadata for the UI: `(bare_key, label, native_max, direct_output)`.
pub const PREBUILT_META: &[(&str, &str, u32, bool)] = &[
    ("pass_shoot_bias", "Shoot/Pass Bias (FWD)", 15, true),
    ("pass_shoot_bias_def", "Shoot/Pass Bias (DEF)", 15, true),
    ("def_off_bias", "Defense/Offense Bias", 15, true),
    ("pass_carry_bias", "Pass/Carry Bias", 15, true),
    ("vezina_consistency", "Goalie Consistency (Vezina)", 2, true),
    ("gp_durability", "Durability (GP%)", 20, false),
    ("shooting_pct", "Shooting Pct (NHL)", 30, false),
];
