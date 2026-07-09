//! Prebuilt formulas — the career/stats-driven components (biases, durability, shooting %, goalie
//! consistency). Ported verbatim from the prior app's `ratings.rs`; the math is documented in
//! `docs/reference/fhm-ea-formula.md` §4. Pure functions over a [`StatsContext`] + FHM values.

use std::collections::BTreeMap;

/// Stats context fed into prebuilt-formula evaluation. Populated by the caller from the player's
/// career stats (NHL regular-season) + identity.
#[derive(Debug, Clone, Default)]
pub struct StatsContext {
    /// Best-position label for the shots-per-game lookup: `"LW"`/`"C"`/`"RW"`/`"D"`/`"G"`/`"F"`.
    pub position: String,
    /// Career-wide NHL regular-season goals / assists / games (shoot-pass + def-off biases).
    pub career_g_pro: u32,
    pub career_a_pro: u32,
    pub career_gp_pro: u32,
    /// Most-recent NHL regular season: games / goals, and that team's schedule length.
    pub recent_gp: u32,
    pub recent_goals: u32,
    pub recent_team_schedule: u32,
    /// Normalised `"<last> <first>"` key for the Vezina lookup.
    pub name_key: String,
}

/// Position-specific NHL-average shots on goal per game (2024-25). Position-neutralises finishing.
fn shots_per_game_for_position(pos: &str) -> f64 {
    match pos {
        "LW" => 1.75,
        "C" => 1.62,
        "RW" => 1.79,
        "LD" | "RD" | "D" => 1.28,
        _ => 1.72,
    }
}

/// Evaluate one prebuilt formula → value on its native scale (0..native_max), or `None` when a
/// required input is missing.
pub fn eval_prebuilt(
    pb_key: &str,
    ctx: &StatsContext,
    fhm_values: &BTreeMap<String, f64>,
) -> Option<f64> {
    match pb_key {
        "pass_shoot_bias" => {
            let bias = 8.0 - (ctx.career_g_pro as f64 / 5.0) + (ctx.career_a_pro as f64 / 10.0);
            Some(bias.round().clamp(0.0, 15.0))
        }
        "pass_shoot_bias_def" => {
            let bias = 8.0 - (ctx.career_g_pro as f64 / 3.0) + (ctx.career_a_pro as f64 / 10.0);
            Some(bias.round().clamp(0.0, 15.0))
        }
        "def_off_bias" => {
            if ctx.career_gp_pro == 0 {
                return Some(8.0);
            }
            let ppg = (ctx.career_g_pro as f64 + ctx.career_a_pro as f64) / ctx.career_gp_pro as f64;
            let bias = 8.0 + (ppg - 0.5) * 10.0;
            Some(bias.round().clamp(0.0, 15.0))
        }
        "vezina_consistency" => {
            let wins = vezina_count(&ctx.name_key);
            let tier = if wins >= 3 {
                2.0
            } else if wins >= 1 {
                1.0
            } else {
                0.0
            };
            Some(tier)
        }
        "pass_carry_bias" => {
            let ph = fhm_values.get("Puck Handling").copied()?;
            let sp = fhm_values.get("Speed").copied()?;
            let pa = fhm_values.get("Passing").copied()?;
            let hs = fhm_values.get("Hockey Sense").copied()?;
            let carry = (ph + sp) / 2.0;
            let pass = (pa + hs) / 2.0;
            let bias = 8.0 + (carry - pass) / 20.0 * 14.0;
            Some(bias.round().clamp(0.0, 15.0))
        }
        "gp_durability" => {
            if ctx.recent_team_schedule == 0 {
                return None;
            }
            Some((ctx.recent_gp as f64 / ctx.recent_team_schedule as f64 * 20.0).clamp(0.0, 20.0))
        }
        "shooting_pct" => {
            let spg = shots_per_game_for_position(&ctx.position);
            let est = ctx.recent_gp as f64 * spg;
            if est <= 0.0 || ctx.recent_goals == 0 {
                return None;
            }
            Some(((ctx.recent_goals as f64) / est * 100.0).min(30.0))
        }
        _ => None,
    }
}

/// Vezina-win count for a player's normalised name key (0 when absent).
pub fn vezina_count(name_key: &str) -> u32 {
    VEZINA_WINNERS
        .iter()
        .find(|(k, _)| *k == name_key)
        .map(|(_, c)| *c)
        .unwrap_or(0)
}

/// Normalise `(first, last)` → `"<last> <first>"`: lowercase, ASCII-fold diacritics, collapse
/// `-`/`'`/`.` + whitespace to single spaces. Matches the hard-coded [`VEZINA_WINNERS`] key form.
pub fn name_key(first: &str, last: &str) -> String {
    let combined = format!("{last} {first}");
    let mut out = String::with_capacity(combined.len());
    let mut last_space = true;
    for c in combined.chars() {
        let mapped: Option<char> = match c {
            'a'..='z' | '0'..='9' => Some(c),
            'A'..='Z' => Some(c.to_ascii_lowercase()),
            ' ' | '\t' | '\n' | '-' | '\'' | '\u{2019}' | '.' => Some(' '),
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => Some('a'),
            'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' | 'Ā' => Some('a'),
            'é' | 'è' | 'ê' | 'ë' | 'ě' | 'ē' => Some('e'),
            'É' | 'È' | 'Ê' | 'Ë' | 'Ě' | 'Ē' => Some('e'),
            'í' | 'ì' | 'î' | 'ï' | 'ī' => Some('i'),
            'Í' | 'Ì' | 'Î' | 'Ï' | 'Ī' => Some('i'),
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ō' => Some('o'),
            'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' | 'Ō' => Some('o'),
            'ú' | 'ù' | 'û' | 'ü' | 'ū' => Some('u'),
            'Ú' | 'Ù' | 'Û' | 'Ü' | 'Ū' => Some('u'),
            'š' | 'Š' => Some('s'),
            'č' | 'Č' => Some('c'),
            'ć' | 'Ć' => Some('c'),
            'ř' | 'Ř' => Some('r'),
            'ž' | 'Ž' => Some('z'),
            'ý' | 'Ý' => Some('y'),
            'ñ' | 'Ñ' => Some('n'),
            'ø' | 'Ø' => Some('o'),
            'æ' | 'Æ' => Some('a'),
            'ß' => Some('s'),
            _ => None,
        };
        if let Some(m) = mapped {
            if m == ' ' {
                if !last_space {
                    out.push(' ');
                    last_space = true;
                }
            } else {
                out.push(m);
                last_space = false;
            }
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Career Vezina winners → count, keyed by [`name_key`] form. Hard-coded (FHM has no Vezina data).
const VEZINA_WINNERS: &[(&str, u32)] = &[
    ("plante jacques", 7),
    ("durnan bill", 6),
    ("hasek dominik", 6),
    ("dryden ken", 5),
    ("brodeur martin", 4),
    ("larocque michel", 4),
    ("sawchuk terry", 4),
    ("thompson tiny", 4),
    ("esposito tony", 3),
    ("hainsworth george", 3),
    ("hall glenn", 3),
    ("hellebuyck connor", 3),
    ("roy patrick", 3),
    ("belfour ed", 2),
    ("bobrovsky sergei", 2),
    ("bower johnny", 2),
    ("brimsek frank", 2),
    ("broda turk", 2),
    ("gardiner charlie", 2),
    ("hodge charlie", 2),
    ("parent bernie", 2),
    ("thomas tim", 2),
    ("worsley gump", 2),
    ("barrasso tom", 1),
    ("carey jim", 1),
    ("chabot lorne", 1),
    ("dejordy denis", 1),
    ("edwards don", 1),
    ("fleury marc andre", 1),
    ("fuhr grant", 1),
    ("giacomin ed", 1),
    ("herron denis", 1),
    ("hextall ron", 1),
    ("holtby braden", 1),
    ("kerr dave", 1),
    ("kiprusoff miikka", 1),
    ("kolzig olaf", 1),
    ("lindbergh pelle", 1),
    ("lumley harry", 1),
    ("lundqvist henrik", 1),
    ("miller ryan", 1),
    ("mowers johnny", 1),
    ("peeters pete", 1),
    ("price carey", 1),
    ("rask tuukka", 1),
    ("rinne pekka", 1),
    ("rollins al", 1),
    ("sauve bob", 1),
    ("sevigny richard", 1),
    ("shesterkin igor", 1),
    ("smith billy", 1),
    ("smith gary", 1),
    ("smith normie", 1),
    ("theodore jose", 1),
    ("ullmark linus", 1),
    ("vachon rogie", 1),
    ("vanbiesbrouck john", 1),
    ("vasilevskiy andrei", 1),
    ("villemure gilles", 1),
    ("worters roy", 1),
];
