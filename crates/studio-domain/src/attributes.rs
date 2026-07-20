//! Player **Attributes / Ratings** presentation vocabulary + view-model.
//!
//! The `dSvy` (skater AI) and `mHuy` (goalie AI) tables each carry ~25 named rating fields, plus a
//! few special fields — three 4-bit skater biases, the **potential/growth** fields (`QoMA` +
//! `mBef`), the Player/Goalie Style enum (`QgFs`), and 32 `Trait N` slots. This module owns the
//! *presentation vocabulary*
//!
//! ## Era-spanning rating classification (metadata-driven, not id-driven)
//!
//! Ratings are recognised by their **metadata name**, not by field id, because EA renamed the whole
//! family at NHL 14 — `p_speed`/`g_gsh`… (NHL 10–13) became `c_speed`/`c_gsh`… — assigning new
//! four-char ids in the same stroke while keeping the name *suffixes* stable (verified against every
//! anchor meta, 2026-07-03). The value **scale** likewise comes from the field's own declared range:
//! PS3/X360 titles store ratings 6-bit `0..63` (display/edit `+36` → the 36..99 scale; verified
//! in-game for the 14/15/Legacy era, provisional for 10–13), while **every PS4 title stores them
//! 7-bit `0..99` — the human value directly, no offset**. Classifying from the loaded metadata makes
//! both eras (and both scales) correct with no per-title tables — the same evidence-over-assumption
//! rule as the capability probe.
//!
//! Potential fields — EA meta names them `growthtier`/`growthletter`, but they ARE the in-game
//! scouting-potential display (the prior React app surfaced them as "Potential Range"/"Potential
//! Accuracy" and it worked in-game — a separate concept from NHLViewNG's numeric "Potential", which is
//! `c_potential`/`tEok`; different apps, different valid fields):
//! * `QoMA` = `growthtier` → **Potential Range** (stars). Operator-anchored: `QoMA=1` = Crosby = 5.0★
//!   (best); higher raw = fewer stars via `(11-raw)/2`. Data-confirmed direction: tier 1 =
//!   generational, 2 = elite incl. blue-chip prospects (Ovechkin, Stamkos, Seguin, Nugent-Hopkins),
//!   down to ~2.5★ for depth. `raw 0` = growth-off (99-ovr legends + generic filler) → "None".
//! * `mBef` = `growthletter` → **Potential Accuracy** (scouting certainty). `{1,2}=High {3,4}=Medium
//!   {5,6}=Low`, `0`="None". Data-confirmed: certainty rises with age (letter 1-2 ≈ 25 y/o vets = High;
//!   5-6 ≈ 19-20 y/o prospects = Low), matching a scouting-precision reading.
//!
//! Both decodes are operator-anchored + data-direction-confirmed but the exact per-value star count is
//! not independently verified — present as provisional. `tEok`=`c_potential` (NHLViewNG's numeric
//! "Potential") is a career-mode field, dormant in the exhibition roster (two-valued 5/30 sentinel,
//! `leagueskaterai` table absent), so it stays in the raw all-fields list only, not the headline.
//! (human labels, display order, the potential/accuracy/active decodes, and the trait dictionary) and
//! assembles a display-ready [`PlayerAttributes`] from the companion field views.
//!
//! The rating scale itself is applied upstream in [`crate::view::read_field`] via
//! [`rating_offset`], so each rating [`FieldView`] already carries the correct `display` and
//! friendly `edit_offset`; this module only adds names, ordering, and the derived-field
//! vocabularies.
//!
//! **Provenance (verify before asserting — Hard rule 3).** Labels, ordering, and the trait dictionary
//! are sourced from the prior app's operator research corpus (an NHL 14 field XML + a 2026 EBOOT
//! debug-string decryption; Operation Sports thread #631755). Trait confidence is carried through so
//! the UI can visibly hedge the weaker guesses. Re-verify against our own data before treating any
//! specific label as fact.

use crate::view::FieldView;
use tdb_core::TdbFile;
use tdb_relations::Position;

/// `true` when this database uses NHL07's gen6 style schema — the skater `p_style` (`mQoE`) is a
/// **3-bit** field (0..7). NHL 10→13 carry the same field id but a **4-bit** (0..15) code space with
/// a larger, still-unverified vocabulary, so the NHL07 style vocabulary and its position split apply
/// only here; NHL 14+ use `QgFs` instead. Keyed on the field shape, never the title id.
pub(crate) fn is_gen6_style_schema(db: &TdbFile) -> bool {
    db.table_canonical("dSvy")
        .and_then(|t| t.fields.iter().find(|f| f.id.canonical(db.endian) == "mQoE"))
        .map(|f| f.bit_width == 3)
        .unwrap_or(false)
}

/// Lock the Player Style vocabulary (`dSvy.QgFs`) to the player's position: the 4-bit code is
/// position-split — raw `1..=4` are defenseman roles, `5..=10` forward roles (see
/// `defs/enums/player-style.toml`). Ineligible roles are dropped from the dropdown so a forward
/// style can't be staged on a defenseman (or vice versa); the *stored* value keeps its entry even
/// when ineligible, so a mismatched style stays visible and correctable instead of being silently
/// re-labelled. Goalie style (`mHuy.QgFs`) has its own vocabulary — callers only pass skater
/// field sets here.
pub(crate) fn restrict_style_options(ratings: &mut [FieldView], position: Position) {
    let Some(style) = ratings
        .iter_mut()
        .find(|f| STYLE_FIELD_IDS.contains(&f.studio_field.as_str()))
    else {
        return;
    };
    // The position-split code ranges differ by schema: NHL07 gen6 `p_style` (`mQoE`) packs
    // defenseman roles into 1..=2 (Defensive/Offensive) and forward roles into 3..=7
    // (Grinder..Dangler); the modern `QgFs` uses 1..=4 / 5..=10. Verified NHL07 against the roster
    // (DiPenta=1, Campbell=2, Drake=3, Weight=4, Guerin=5, Tkachuk=6, Comrie=7).
    let gen6 = style.studio_field == "mQoE" && style.range_high == Some(7);
    let eligible: std::ops::RangeInclusive<i64> = match (gen6, position == Position::Defense) {
        (true, true) => 1..=2,
        (true, false) => 3..=7,
        (false, true) => 1..=4,
        (false, false) => 5..=10,
    };
    let current = style.decoded;
    style
        .enum_options
        .retain(|(raw, _)| eligible.contains(raw) || Some(*raw) == current);
}

/// NHL07's **birthplace / nationality** country codes (`ubPc.Lelh` nationality and the nation range
/// of `ubPc.QvxI` "Origin"). It is a *different* enumeration from the modern `nationality.toml` — the
/// two share the North-American + Nordic low codes but diverge from 73 up (modern labels code 75
/// "Lithuania", but NHL07 stores Russians there), so the modern vocabulary actively mislabels NHL07's
/// European players. Anchored to the NHL07 roster: every code below is confirmed in-game by the user
/// or fixed by a well-known player's documented birthplace (the last four are *birthplaces* whose
/// players carry other nationalities — Nolan/Regehr are Canadian, Park American, Kölzig German). The
/// game's Origin dropdown is alphabetical and does NOT follow these stored codes, so codes are pinned
/// from player data, not list order. Code 78 (Belgium, single player Pellegrims) is inferred.
const GEN6_COUNTRIES: &[(i64, &str)] = &[
    (0, "Canada"),           // ✓ Crosby, Sakic
    (14, "United States"),   // ✓ Amonte
    (66, "Denmark"),         // Christensen/Christiansen
    (67, "Finland"),         // ✓ Aalto
    (68, "Norway"),          // Boye/Brathen
    (69, "Sweden"),          // ✓ Alfredsson
    (70, "Belarus"),         // Arkatov/Bulakhov
    (72, "Kazakhstan"),      // ✓ Antropov
    (73, "Latvia"),          // ✓ user-confirmed (Apsitis)
    (74, "Lithuania"),       // ✓ user-confirmed (Kasparaitis, Zubrus)
    (75, "Russia"),          // ✓ user-confirmed (Afinogenov)
    (76, "Ukraine"),         // ✓ user-confirmed (Bilik, Bobyk)
    (77, "Austria"),         // ✓ user-confirmed (Divis)
    (78, "Belgium"),         // inferred (Pellegrims) — unconfirmed
    (79, "Czech Republic"),  // ✓ user-confirmed (Barinka)
    (80, "Great Britain"),   // ✓ user-confirmed (Bentley/Blair)
    (81, "France"),          // ✓ user-confirmed
    (82, "Germany"),         // ✓ user-confirmed
    (84, "Italy"),           // ✓ user-confirmed
    (85, "Northern Ireland"),// Owen Nolan (born Belfast)
    (86, "Poland"),          // ✓ user-confirmed
    (88, "Slovakia"),        // ✓ user-confirmed (Bartecko, Bondra)
    (89, "Switzerland"),     // ✓ user-confirmed
    (92, "Brazil"),          // Robyn Regehr (born Recife)
    (96, "Japan"),           // ✓ user-confirmed
    (100, "South Africa"),   // Olaf Kölzig (born Johannesburg)
    (101, "South Korea"),    // Richard Park (born Seoul)
];

/// NHL07's nationality vocabulary (see [`GEN6_COUNTRIES`]).
pub(crate) fn gen6_nationality() -> Vec<(i64, String)> {
    GEN6_COUNTRIES.iter().map(|(r, l)| (*r, l.to_string())).collect()
}

/// The highest NHL07 birth-state code that is a North-American *state/province* (`ubPc.QvxI`): the
/// "Origin" list runs Canadian provinces + US states through 65, then switches to *nations* from 66
/// up. The state range matches the modern vocabulary (verified: Crosby=7 Nova Scotia, Iginla=1
/// Alberta); only the nation range diverges (see [`GEN6_COUNTRIES`]).
const GEN6_STATE_MAX: i64 = 65;

/// Apply NHL07's gen6 geography vocabulary to a field's dropdown options, in place. Both nationality
/// (`Lelh`) and the birth-state "Origin" nation range (`QvxI` ≥ 66) use NHL07's own country table —
/// the modern vocabulary mislabels them (code 75 is Russia, not "Lithuania"). For `QvxI` the modern
/// North-American state labels (≤ 65) are kept and only the nation range is replaced. No-op unless
/// this is the gen6 schema and the field is one of the two.
pub(crate) fn apply_gen6_geo_vocab(
    studio_table: &str,
    studio_field: &str,
    gen6: bool,
    options: &mut Vec<(i64, String)>,
) {
    if !gen6 || studio_table != "ubPc" {
        return;
    }
    match studio_field {
        "Lelh" => *options = gen6_nationality(),
        "QvxI" => {
            // Keep the North-American states; replace the (mislabeled modern) nation range with
            // NHL07's own country codes. Nations NHL07 uses but we haven't identified yet drop out
            // and render raw rather than showing a wrong country.
            options.retain(|(r, _)| *r <= GEN6_STATE_MAX);
            // NHL07 orders DC before Delaware — the two are swapped vs the modern vocab (user-verified).
            for (raw, label) in [(22, "USA, District of Columbia"), (23, "USA, Delaware")] {
                if let Some(e) = options.iter_mut().find(|(r, _)| *r == raw) {
                    e.1 = label.to_string();
                }
            }
            for (raw, label) in gen6_nationality() {
                if raw > GEN6_STATE_MAX && !options.iter().any(|(r, _)| *r == raw) {
                    options.push((raw, label));
                }
            }
            options.sort_by_key(|(r, _)| *r);
        }
        _ => {}
    }
}

/// The player/goalie **style** (archetype) field id across schema variants: `QgFs` on NHL 11→21, or
/// the gen6 NHL07 companions `mQoE` (skater `p_style`, 0..7) / `wJus` (goalie `g_style`, 0..3).
pub(crate) const STYLE_FIELD_IDS: &[&str] = &["QgFs", "mQoE", "wJus"];

/// The style [`FieldView`] on a companion's ratings, whichever id this schema carries.
pub(crate) fn style_field<'a>(ratings: &'a [FieldView]) -> Option<&'a FieldView> {
    ratings.iter().find(|f| STYLE_FIELD_IDS.contains(&f.studio_field.as_str()))
}

/// The rating-name suffix of an AI-table metadata name: strips the era prefix — `p_` (skaters
/// 10–13), `g_` (goalies 10–13), or `c_` (14 onward, both kinds). `None` for unprefixed names
/// (biases, growth/trait/skill fields, join keys), which are never ratings.
pub(crate) fn rating_suffix(meta_name: &str) -> Option<&str> {
    ["p_", "g_", "c_"]
        .iter()
        .find_map(|p| meta_name.strip_prefix(p))
}

/// Prefixed AI-table fields that are **not** 36..99/0..99 ratings: fighting *frequency* (0..3),
/// the style enums, the paddle-down flag, and the dormant career-mode numeric potential
/// (`c_potential`/`tEok` — NHLViewNG's "Potential", two-valued in shipped rosters; stays in the raw
/// all-fields list only). The range check below excludes most of these already; the suffix list
/// keeps the 6-bit `potential` out and documents the intent.
const NON_RATING_SUFFIXES: &[&str] = &["fighting", "style", "paddledown", "potential"];

/// Classify one companion-table field as a rating **from its own metadata** and return the
/// display/edit offset: `Some(36)` for 6-bit `0..63` storage (PS3/X360 — shows/edits on the 36..99
/// scale), `Some(0)` for `0..99` storage (PS4 — the stored value *is* the rating), `None` for
/// everything that isn't a rating. See the module docs for the era/provenance details.
pub(crate) fn rating_offset(
    studio_table: &str,
    meta_name: &str,
    range_low: Option<i64>,
    range_high: Option<i64>,
) -> Option<i64> {
    if !matches!(studio_table, "dSvy" | "mHuy") {
        return None;
    }
    let suffix = rating_suffix(meta_name)?;
    if NON_RATING_SUFFIXES.contains(&suffix) {
        return None;
    }
    match (range_low?, range_high?) {
        (0, 63) => Some(36),
        (0, 99) => Some(0),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// View-model
// ---------------------------------------------------------------------------

/// The full, display-ready Attributes/Ratings view for one player.
#[derive(Clone, Debug, Default)]
pub struct PlayerAttributes {
    /// `"skater"` | `"goalie"`, or `None` when the player has no ratings companion.
    pub kind: Option<&'static str>,
    /// Overall (`eWUF`, a 6-bit rating).
    pub overall: Option<AttributeView>,
    /// Player/Goalie Style (`QgFs`, real enum vocabulary).
    pub style: Option<AttributeView>,
    /// Potential Range (`QoMA` = `growthtier`): in-game potential as stars (raw 1=5.0★ … , 0="None").
    pub potential_range: Option<AttributeView>,
    /// Potential Accuracy (`mBef` = `growthletter`): scouting certainty (High/Medium/Low, 0="None").
    pub potential_accuracy: Option<AttributeView>,
    /// The curated, ordered attribute list (biases interleaved with ratings).
    pub flat: Vec<AttributeView>,
    /// Active traits decoded from the 32 `Trait N` slots (empty slots skipped, values deduped).
    pub traits: Vec<TraitView>,
    /// The full trait vocabulary as toggleable options, each flagged with whether it's currently
    /// active (occupies a slot) — powers the activate/deactivate checkbox menu.
    pub trait_options: Vec<TraitOption>,
}

/// One entry in the trait checkbox menu: a dictionary trait plus whether the player currently has it.
#[derive(Clone, Debug)]
pub struct TraitOption {
    /// Trait number (1..27 skater, 1..2 goalie).
    pub value: i64,
    pub label: String,
    /// `"confirmed"` | `"strong"` | `"weak"` — how solid the label is.
    pub confidence: &'static str,
    pub note: Option<String>,
    /// True when at least one `Trait N` slot on the record holds this value.
    pub active: bool,
}

/// One labelled attribute cell. Carries the underlying editable [`FieldView`] so the frontend edits it
/// through the ordinary `set_field` + friendly-input path. `field` is `Option` to leave room for a
/// future placeholder (a known attribute whose source field isn't anchored yet); every cell the
/// builder emits today is `Some`.
#[derive(Clone, Debug)]
pub struct AttributeView {
    /// Operator label, e.g. `"Speed"`, `"Off / Def Bias"`.
    pub label: String,
    /// True when this is a 6-bit `+36` rating (drives tier coloring); false for biases/enums/derived.
    pub is_rating: bool,
    /// The underlying field to display/edit; `None` for a not-yet-anchored placeholder.
    pub field: Option<FieldView>,
}

/// One active trait on a player (decoded from a `Trait N` slot).
#[derive(Clone, Debug)]
pub struct TraitView {
    /// The slot field id that held the value (e.g. `"wBhb"` = Trait 0).
    pub slot_field: String,
    /// Slot index 0..31.
    pub slot_index: u32,
    /// Trait number stored in the slot (1..27 for skaters; small goalie vocab).
    pub value: i64,
    /// Best label for the value, or `"Trait #N"` when unmapped.
    pub label: String,
    /// `"confirmed"` | `"strong"` | `"weak"` | `"unknown"` — how solid the label is.
    pub confidence: &'static str,
    /// Optional provenance / example-players tooltip.
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// Synthetic enums (potential / accuracy / active) — surfaced through `read_field`
// ---------------------------------------------------------------------------

/// A synthesized `(raw, label)` vocabulary for a ratings-table field with no TOML enum but a known
/// value→label mapping. Returned to [`crate::view::read_field`] so the field renders (and edits) as a
/// labelled dropdown. Operator-anchored + data-direction-confirmed (see module docs); provisional.
pub(crate) fn synthetic_enum(
    studio_table: &str,
    studio_field: &str,
    gen6_style: bool,
) -> Option<Vec<(i64, String)>> {
    match (studio_table, studio_field) {
        ("dSvy" | "mHuy", "QoMA") => Some(potential_range_options()),
        ("dSvy" | "mHuy", "mBef") => Some(potential_accuracy_options()),
        // NHL07 gen6 player types (verified in-game), only on the 3-bit gen6 schema — NHL 10→13
        // reuse these ids with a wider, unverified code space. Skaters and goalies each have their
        // own vocabulary; the skater set is position-split by [`restrict_style_options`].
        ("dSvy", "mQoE") if gen6_style => Some(skater_style_options()),
        ("mHuy", "wJus") if gen6_style => Some(goalie_style_options()),
        _ => None,
    }
}

/// NHL07 skater `p_style` (`mQoE`, 0..7): defenseman roles 1..=2, forward roles 3..=7. Verified
/// against named players (DiPenta=Defensive, B.Campbell=Offensive, Drake=Grinder, Weight=Playmaker,
/// Guerin=Sniper, Tkachuk=Power Forward, Comrie=Dangler).
fn skater_style_options() -> Vec<(i64, String)> {
    [
        (1, "Defensive"),
        (2, "Offensive"),
        (3, "Grinder"),
        (4, "Playmaker"),
        (5, "Sniper"),
        (6, "Power Forward"),
        (7, "Dangler"),
    ]
    .into_iter()
    .map(|(r, l)| (r, l.to_string()))
    .collect()
}

/// NHL07 goalie `g_style` (`wJus`, 0..3). Verified: Bryzgalov=Butterfly (2), Brathwaite=Hybrid (1),
/// Brodeur/Burke=Stand-Up (0).
fn goalie_style_options() -> Vec<(i64, String)> {
    [(0, "Stand-Up"), (1, "Hybrid"), (2, "Butterfly")]
        .into_iter()
        .map(|(r, l)| (r, l.to_string()))
        .collect()
}

/// Potential Range (`QoMA` = `growthtier`) → star potential. Anchored `raw 1` = 5.0★ (Crosby); the
/// operator interpolation is `(11 - raw)/2` stars for raw 1..10. `raw 0` = growth-off → "None".
fn potential_range_options() -> Vec<(i64, String)> {
    let mut out = vec![(0, "None".to_string())];
    out.extend((1..=10).map(|raw| {
        let stars = (11 - raw) as f64 / 2.0;
        (raw, format!("{stars:.1}\u{2605}")) // e.g. "4.5★"
    }));
    out
}

/// Potential Accuracy (`mBef` = `growthletter`) → scouting certainty. Six internal values collapse to
/// three buckets: `{1,2}=High {3,4}=Medium {5,6}=Low`; `0` = "None" (untracked / grown).
fn potential_accuracy_options() -> Vec<(i64, String)> {
    [(0, "None"), (1, "High"), (2, "High"), (3, "Medium"), (4, "Medium"), (5, "Low"), (6, "Low")]
        .into_iter()
        .map(|(raw, label)| (raw, label.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Skater vocabulary + order
// ---------------------------------------------------------------------------

/// (rating-name suffix, label) for every named skater field, spanning all eras: the shared
/// suffixes (`speed`, `puckcontrol`, …), plus NHL 10's pre-split vocabulary (`checking`,
/// `deflections`, `toughness`) and PS4 18+'s `backskating`. The curated flat list
/// ([`SKATER_ORDER`]) is a subset; uncovered ratings are appended after it.
const SKATER_LABELS: &[(&str, &str)] = &[
    ("overall", "Overall"),
    ("speed", "Speed"),
    ("acceleration", "Acceleration"),
    ("agility", "Agility"),
    ("balance", "Balance"),
    ("strength", "Strength"),
    ("endurance", "Endurance"),
    ("durability", "Durability"),
    ("handeye", "Hand-Eye"),
    ("discipline", "Discipline"),
    ("offawareness", "Offensive Awareness"),
    ("defawareness", "Defensive Awareness"),
    ("aggression", "Aggression"),
    ("poise", "Poise"),
    ("faceoffs", "Faceoffs"),
    ("slapshotpower", "Slap Shot Power"),
    ("slapaccuracy", "Slap Shot Accuracy"),
    ("wristshotpower", "Wrist Shot Power"),
    ("wristaccuracy", "Wrist Shot Accuracy"),
    ("stickchecking", "Stick Checking"),
    ("bodychecking", "Body Checking"),
    ("shotblocking", "Shot Blocking"),
    ("deking", "Deking"),
    ("passing", "Passing"),
    ("puckcontrol", "Puck Control"),
    ("fighting", "Fighting Frequency"),
    ("fightingskill", "Fight Skills"),
    // NHL 10 only — the pre-NHL-11 vocabulary before checking split into body/stick checking.
    ("checking", "Checking"),
    ("deflections", "Deflections"),
    ("toughness", "Toughness"),
    // PS4 NHL 18+ addition.
    ("backskating", "Backskating"),
    // NHL07 gen6-only suffixes (verified in-game names).
    ("injury", "Injury Resist"),
    ("penalty", "Penalty Resist"),
    ("herorating", "Intensity"),
];

/// One slot of the curated skater order: a rating suffix or a bias id, with an optional label
/// override (used when the display label differs from the dictionary entry).
enum SkaterSlot {
    Rating(&'static str, Option<&'static str>),
    /// An AI-shape slider (`off_defbias`/`pass_carrybias`/`shoot_passbias`). Unlike the ratings
    /// these are 4-bit fields on the **0..15** scale, stored raw — and their *ids* are stable
    /// across every anchor title, so they stay id-keyed. Goalies have no biases.
    Bias(&'static str, &'static str),
}

/// The operator-specified skater display order (biases interleaved with ratings).
const SKATER_ORDER: &[SkaterSlot] = &[
    SkaterSlot::Rating("speed", None),
    SkaterSlot::Rating("bodychecking", None),
    SkaterSlot::Rating("puckcontrol", None),
    SkaterSlot::Rating("endurance", None),
    SkaterSlot::Rating("passing", None),
    SkaterSlot::Bias("vslh", "Shoot / Pass Bias"),
    SkaterSlot::Rating("slapshotpower", None),
    SkaterSlot::Rating("slapaccuracy", None),
    SkaterSlot::Rating("wristshotpower", None),
    SkaterSlot::Rating("wristaccuracy", None),
    SkaterSlot::Rating("agility", None),
    SkaterSlot::Rating("acceleration", None),
    SkaterSlot::Rating("balance", None),
    SkaterSlot::Rating("faceoffs", Some("Face-Offs")),
    SkaterSlot::Rating("durability", None),
    SkaterSlot::Rating("deking", None),
    SkaterSlot::Rating("aggression", Some("Aggressiveness")),
    SkaterSlot::Rating("poise", None),
    SkaterSlot::Rating("handeye", None),
    SkaterSlot::Rating("shotblocking", None),
    SkaterSlot::Rating("offawareness", None),
    SkaterSlot::Rating("defawareness", None),
    SkaterSlot::Rating("discipline", None),
    SkaterSlot::Bias("YskE", "Defense / Offense Bias"),
    SkaterSlot::Bias("YRab", "Pass / Carry Bias"),
];

// ---------------------------------------------------------------------------
// Goalie vocabulary + order
// ---------------------------------------------------------------------------

/// (rating-name suffix, label) for every named goalie field, spanning `g_*` (10–13) and `c_*`
/// (14+); `floppiness` is NHL 10 only.
const GOALIE_LABELS: &[(&str, &str)] = &[
    ("overall", "Overall"),
    ("speed", "Speed"),
    ("agility", "Agility"),
    ("aggression", "Aggression"),
    ("vision", "Vision"),
    ("positioning", "Positioning"),
    ("gsh", "Glove High"),
    ("gsl", "Glove Low"),
    ("ssh", "Stick High"),
    ("ssl", "Stick Low"),
    ("5hole", "Five Hole"),
    ("reboundcontrol", "Rebound Control"),
    ("shotrecover", "Shot Recovery"),
    ("breakaway", "Breakaway"),
    ("consistency", "Consistency"),
    ("passing", "Passing"),
    ("pokecheck", "Poke Check"),
    ("puckplayfrequency", "Puck Playing Freq"),
    ("paddledown", "Paddle Down"),
    ("poise", "Poise"),
    ("endurance", "Endurance"),
    ("durability", "Durability"),
    ("fighting", "Fighting Frequency"),
    ("floppiness", "Floppiness"),
    // NHL07 gen6-only goalie suffix.
    ("intensity", "Intensity / Clutch"),
];

/// Goalie display order (Reflexes → Save Locations → Save Handling → Puck Handling → Mental/Physical),
/// flattened to a single suffix list. Goalies carry no biases. `paddledown`/`fighting` are shown but
/// classify non-rating (flag / 0..3 frequency), so they render raw and untinted.
const GOALIE_ORDER: &[&str] = &[
    "speed", "agility", "aggression", "vision", "positioning", "gsh", "gsl", "ssh", "ssl", "5hole",
    "reboundcontrol", "shotrecover", "breakaway", "consistency", "passing", "pokecheck",
    "puckplayfrequency", "paddledown", "poise", "endurance", "durability", "fighting",
];

fn lookup(map: &[(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    map.iter().find(|(k, _)| *k == key).map(|(_, label)| *label)
}

/// The operator label for a rating-name suffix on the given companion table, if catalogued.
pub(crate) fn rating_label(studio_table: &str, suffix: &str) -> Option<&'static str> {
    match studio_table {
        "dSvy" => lookup(SKATER_LABELS, suffix),
        "mHuy" => lookup(GOALIE_LABELS, suffix),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Derived skater Overall (the depth-chart / list number)
// ---------------------------------------------------------------------------

/// Per-attribute terms of the skater Overall approximation, keyed by rating-name **suffix**:
/// `(suffix, defenseman_coefficient, forward_weight)`. Defensemen use a linear regression
/// (`D_INTERCEPT + Σ coeff·value`); forwards a weighted average whose weights sum to 65
/// (`FWD_SCALE · (Σ weight·value)/65 + FWD_INTERCEPT`). Values are EA **display** ratings
/// (36..99 — or the stored 0..99 on PS4), so the formula is era-portable.
///
/// Provenance: empirically recovered in the prior app by regressing stored `eWUF` Overalls on the
/// rating fields across `default.db` (the goalie fit reported R²=0.967 / RMSE 2.2; the skater fit
/// is the same programme). Ported term-for-term 2026-07-05; the coefficients are a
/// collinearity-blurred *equivalent* of EA's authored weights — the predictor is accurate, but
/// read individual weights as a ranking, not gospel.
const SKATER_OVERALL_TERMS: &[(&str, f64, f64)] = &[
    ("speed", 0.023699, 3.0),
    ("bodychecking", 0.084108, 2.0),
    ("endurance", 0.010734, 1.0),
    ("puckcontrol", -0.029, 2.0),
    ("passing", 0.058153, 5.0),
    ("slapshotpower", 0.071821, 2.0),
    ("slapaccuracy", 0.073114, 2.0),
    ("wristshotpower", 0.014244, 3.0),
    ("wristaccuracy", -0.00035, 4.0),
    ("agility", 0.089079, 3.0),
    ("strength", 0.243921, 3.0),
    ("acceleration", 0.048991, 3.0),
    ("balance", -0.01611, 3.0),
    ("faceoffs", 0.023246, 1.0),
    ("durability", -0.00716, 1.0),
    ("deking", 0.037535, 1.0),
    ("aggression", -0.00211, 1.0),
    ("poise", 0.086034, 2.0),
    ("handeye", -0.02652, 1.0),
    ("shotblocking", 0.157127, 1.0),
    ("offawareness", 0.102197, 7.0),
    ("defawareness", 0.557359, 9.0),
    ("discipline", 0.008544, 1.0),
    ("fightingskill", -0.0169, 1.0),
    ("stickchecking", 0.270786, 3.0),
];
const SKATER_OVERALL_D_INTERCEPT: f64 = -74.3907;
const SKATER_OVERALL_FWD_SCALE: f64 = 1.9165;
const SKATER_OVERALL_FWD_DIVISOR: f64 = 65.0;
const SKATER_OVERALL_FWD_INTERCEPT: f64 = -78.725;

/// Skater Overall from per-attribute **display** values (36..99), resolved by
/// `value_for(rating suffix)`. Missing attributes count as 0. Rounded + clamped to 1..99.
/// `is_defenseman` selects the D regression over the forward weighted-average. (Goalies don't use
/// a formula — their stored `overall` field is read directly.)
pub(crate) fn skater_overall(value_for: impl Fn(&str) -> Option<f64>, is_defenseman: bool) -> i64 {
    let mut acc = 0.0f64;
    for (suffix, coeff, weight) in SKATER_OVERALL_TERMS {
        let v = value_for(suffix).unwrap_or(0.0);
        acc += if is_defenseman { coeff * v } else { weight * v };
    }
    let ovr = if is_defenseman {
        SKATER_OVERALL_D_INTERCEPT + acc
    } else {
        SKATER_OVERALL_FWD_SCALE * (acc / SKATER_OVERALL_FWD_DIVISOR) + SKATER_OVERALL_FWD_INTERCEPT
    };
    (ovr.round() as i64).clamp(1, 99)
}

// ---------------------------------------------------------------------------
// Trait slots + dictionaries
// ---------------------------------------------------------------------------

/// The 32 `Trait N` slot field ids in slot-index order (Trait 0 → Trait 31). Identical on both
/// `dSvy` and `mHuy`; only the value→label *vocabulary* differs by kind. `pub(crate)` so the edit
/// session can read/write the slots for the activate/deactivate toggle.
pub(crate) const TRAIT_SLOT_ORDER: &[&str] = &[
    "wBhb", "juxV", "inAr", "dsEd", "zIti", "gLJw", "hSkO", "eRoU", "qSRN", "ZRhX", "WGBo", "tNFW",
    "oEgI", "nzwy", "dvlx", "mspz", "jBsh", "sCIr", "kFli", "vGpo", "BjuX", "egeN", "LpDr", "Aqzp",
    "CSGe", "dZqo", "IQnO", "DLjg", "vweV", "UrOb", "cIfK", "nDvA",
];

/// (value, label, confidence, note) for skater traits (`dSvy`). Values 1..17 + 19 are EBOOT-confirmed;
/// 18/20/21/23/27 are strongly anchored by player samples; 22/24/25/26 are best-guess (weak).
const SKATER_TRAITS: &[(i64, &str, &str, &str)] = &[
    (1, "Net Presence", "confirmed", "Forwards who hang in front for screens / deflections (Backes, Kesler, Bertuzzi)."),
    (2, "Net Agitator", "confirmed", "Forwards who screen / distract the goalie (Ott, Lucic, Hartnell)."),
    (3, "Setup Man Low", "confirmed", "Forwards who set up down low looking for slot shooters (Backlund, Stepan, Thornton)."),
    (4, "Shooter Slot", "confirmed", "Forwards positioned in the slot when set up in the O-zone (Alfredsson, Benn, Voracek)."),
    (5, "Rush Joining D", "confirmed", "Defensemen who lead the rush (Del Zotto, Faulk, Letang)."),
    (6, "Dump & Chaser", "confirmed", "Grinder forwards; strong board play (Fisher, Callahan, Neil)."),
    (7, "Shoots On The Rush", "confirmed", "Forwards more likely to shoot off the rush than dump in (Sharp, Setoguchi, Neal)."),
    (8, "Drives The Net", "confirmed", "Forwards who drive hard to the net (Crosby, Clarkson, Lucic)."),
    (9, "Pinching D", "confirmed", "Defensemen willing to risk pinches in the O-zone (Beauchemin, Goligoski, Leddy)."),
    (10, "Finishes Hits", "confirmed", "Any skater; finishes the hit after the puck is gone (Clutterbuck, Martin, Hamonic; Matt Cooke is canonical)."),
    (11, "One Timer Specialist", "confirmed", "Forwards who set up cleanly for one-timers (Stamkos, Whitney, Parise); enables the one-knee animation."),
    (12, "Puck Protector", "confirmed", "EBOOT debug-list name (superseded the OP's 'Passes Through Traffic' guess)."),
    (13, "Offensive Opportunist", "confirmed", "Cherry-pickers who anticipate the quick breakaway pass (Gaborik, Semin, P. Kane)."),
    (14, "Non-Physical", "confirmed", "Any skater; rarely throws hits, positions away from contact (Raymond, Bouwmeester, Zanon)."),
    (15, "Hip Checker", "confirmed", "Defensemen who hip-check zone-entry forwards (Orpik, Emelin, Hamhuis, Ballard)."),
    (16, "Dangler", "confirmed", "EBOOT debug-list name (superseded the OP's 'Puck Protectors' guess)."),
    (17, "Hard Backchecker", "confirmed", "Forwards who play with energy and excel defensively (Brown, Couturier, Bergeron)."),
    (18, "Lazy Backchecker", "strong", "EBOOT-listed; debug-string position sits between Hard Backchecker and Peripheral Player, so almost certainly slot 18."),
    (19, "Peripheral Player", "confirmed", "Forwards who prefer the corners and shy from net traffic (Tlusty, Vrbata, Parenteau)."),
    (20, "Shooter Slot (Defense)", "strong", "Defensemen who carry the puck into the slot to shoot (Gonchar, Keith, Doughty); 18 known players, all offensive D."),
    (21, "Puck Moving Defenseman", "strong", "Defensemen who QB the power play (Doughty, Visnovsky, Ekman-Larsson, Chara, B. Campbell)."),
    (22, "Perimeter Sniper?", "weak", "Best guess (seamor3 / forum post #18). Observed on Frolik, Radulov, Bozak, Seguin, Kovalchuk."),
    (23, "Explosive Speed", "strong", "EBOOT name 'Speedster'. Any skater; elite first step (Higgins, Marleau, Havlat)."),
    (24, "Risky Dangler?", "weak", "Best guess (seamor3 / forum post #18). Observed on Eberle, Oshie, Hemsky, Raymond."),
    (25, "Risk Taker, D-Zone?", "weak", "Best guess (seamor3 / forum post #18). Observed on Antropov, Filppula, Eberle, H. Sedin."),
    (26, "Risk Taker, O-Zone?", "weak", "Best guess (seamor3 / forum post #18). Only 4 known holders: Stempniak, Seguin, D. Sedin, Kesler."),
    (27, "Booming Shot From Point", "strong", "EBOOT name 'Big Shooter'. Heavy point shots, distinct animation (Souray, Garrison; forwards Ovechkin, Kovalchuk)."),
];

/// (value, label, confidence, note) for goalie traits (`mHuy`). Only two entries are community-named;
/// everything else is unknown-vocabulary territory.
const GOALIE_TRAITS: &[(i64, &str, &str, &str)] = &[
    (1, "Feisty?", "weak", "EBOOT debug name; assigned to value 1 by seamor3 (Emery / Price / Varlamov / Mike Smith carry it)."),
    (2, "Floppy?", "weak", "EBOOT debug name; assigned to value 2 by seamor3 (Fleury / Niemi / Luongo carry it)."),
];

fn lookup_trait(is_skater: bool, value: i64) -> Option<(&'static str, &'static str, &'static str)> {
    let table = if is_skater { SKATER_TRAITS } else { GOALIE_TRAITS };
    table
        .iter()
        .find(|(v, ..)| *v == value)
        .map(|(_, label, conf, note)| (*label, *conf, *note))
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Assemble the display-ready [`PlayerAttributes`] from a player's ratings companion field views.
/// `ratings_table` is `"dSvy"` (skater) or `"mHuy"` (goalie); anything else (or empty `ratings`)
/// yields the `kind: None` empty view.
pub fn build_player_attributes(ratings: &[FieldView], ratings_table: &str) -> PlayerAttributes {
    let is_skater = ratings_table == "dSvy";
    let is_goalie = ratings_table == "mHuy";
    if ratings.is_empty() || (!is_skater && !is_goalie) {
        return PlayerAttributes::default();
    }
    let kind = if is_skater { "skater" } else { "goalie" };

    let find = |id: &str| ratings.iter().find(|f| f.studio_field == id).cloned();
    // Rating cells resolve by metadata-name suffix, so one order/label vocabulary spans the
    // p_/g_ (NHL 10–13) and c_ (14+) eras despite the id reassignment between them.
    let find_rating = |suffix: &str| {
        ratings
            .iter()
            .find(|f| rating_suffix(&f.name) == Some(suffix))
            .cloned()
    };
    // `is_rating` (tier coloring) comes from the same metadata classification as the +36/raw scale.
    let cell_is_rating = |f: &FieldView| {
        rating_offset(ratings_table, &f.name, f.range_low, f.range_high).is_some()
    };
    // A headline/derived cell (never tier-colored) bound to an existing field.
    let derived = |id: &str, label: &str| {
        find(id).map(|f| AttributeView {
            label: label.to_string(),
            is_rating: false,
            field: Some(f),
        })
    };

    // Overall (`eWUF`) is intentionally NOT surfaced as an editable cell: it's a derived value we'll
    // populate from a ratings formula later, not hand-edit. It stays in the raw "all rating fields"
    // list for inspection.
    let overall = None;
    // The style/archetype field id varies by schema (`QgFs`, or NHL07's `mQoE`/`wJus`). NHL07 gen6
    // (the 3-bit skater `p_style`) also means: no biases exposed (they're internal AI sliders), so
    // drop them from the list. NHL 10→13 reuse `mQoE` (4-bit) but keep their biases visible.
    let gen6 = style_field(ratings)
        .map(|f| f.studio_field == "mQoE" && f.range_high == Some(7))
        .unwrap_or(false);
    let style = style_field(ratings).cloned().map(|f| AttributeView {
        label: if is_skater { "Player Style" } else { "Goalie Style" }.to_string(),
        is_rating: false,
        field: Some(f),
    });
    // In-game potential (EA field names growthtier/growthletter) → stars + scouting certainty, via the
    // synthesized vocabularies above. Operator-anchored + data-confirmed; see module docs.
    let potential_range = derived("QoMA", "Potential Range");
    let potential_accuracy = derived("mBef", "Potential Accuracy");

    let mut flat: Vec<AttributeView> = Vec::new();
    let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();
    if is_skater {
        for slot in SKATER_ORDER {
            match slot {
                SkaterSlot::Rating(suffix, label_override) => {
                    if let Some(f) = find_rating(suffix) {
                        covered.insert((*suffix).to_string());
                        let is_rating = cell_is_rating(&f);
                        flat.push(AttributeView {
                            label: label_override
                                .or_else(|| lookup(SKATER_LABELS, suffix))
                                .unwrap_or(suffix)
                                .to_string(),
                            is_rating,
                            field: Some(f),
                        });
                    }
                }
                SkaterSlot::Bias(id, label) => {
                    // NHL07 keeps the pass/carry/def biases internal — not player-facing sliders.
                    if gen6 {
                        continue;
                    }
                    if let Some(mut f) = find(id) {
                        // Biases are 4-bit sliders — pin the 0..15 scale so the editor bounds
                        // them there regardless of what the metadata range declares.
                        f.range_low = Some(0);
                        f.range_high = Some(15);
                        flat.push(AttributeView {
                            label: (*label).to_string(),
                            is_rating: false,
                            field: Some(f),
                        });
                    }
                }
            }
        }
    } else {
        for suffix in GOALIE_ORDER {
            if let Some(f) = find_rating(suffix) {
                covered.insert((*suffix).to_string());
                let is_rating = cell_is_rating(&f);
                flat.push(AttributeView {
                    label: lookup(GOALIE_LABELS, suffix).unwrap_or(suffix).to_string(),
                    is_rating,
                    field: Some(f),
                });
            }
        }
    }
    // Ratings the curated order doesn't know (an era's own vocabulary — NHL 10's Checking /
    // Deflections / Toughness / Floppiness, PS4 18+'s Backskating, anything future) are appended
    // so every classified rating in the loaded schema is visible, labelled from the dictionary or,
    // failing that, by its metadata suffix. `overall` stays out of the flat list by design (a
    // derived value — see the note above).
    for f in ratings {
        let Some(suffix) = rating_suffix(&f.name) else {
            continue;
        };
        // `prestige` is NHL07's internal star-power slider, not a player-facing attribute.
        if suffix == "overall" || suffix == "prestige" || covered.contains(suffix) || !cell_is_rating(f) {
            continue;
        }
        covered.insert(suffix.to_string());
        flat.push(AttributeView {
            label: rating_label(ratings_table, suffix).unwrap_or(suffix).to_string(),
            is_rating: true,
            field: Some(f.clone()),
        });
    }

    let traits = build_traits(ratings, is_skater);
    let trait_options = build_trait_options(ratings, is_skater);

    PlayerAttributes {
        kind: Some(kind),
        overall,
        style,
        potential_range,
        potential_accuracy,
        flat,
        traits,
        trait_options,
    }
}

/// Decode the 32 `Trait N` slots into active traits: skip empty (value 0) and missing slots, dedupe by
/// value, and resolve each against the kind-correct dictionary.
fn build_traits(ratings: &[FieldView], is_skater: bool) -> Vec<TraitView> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (slot_index, slot_field) in TRAIT_SLOT_ORDER.iter().enumerate() {
        let Some(value) = ratings
            .iter()
            .find(|f| &f.studio_field == slot_field)
            .and_then(|f| f.decoded)
        else {
            continue;
        };
        if value == 0 || !seen.insert(value) {
            continue;
        }
        let (label, confidence, note) = match lookup_trait(is_skater, value) {
            Some((label, conf, note)) => (label.to_string(), conf, Some(note.to_string())),
            None => (
                format!("Trait #{value}"),
                "unknown",
                Some(if is_skater {
                    "Value outside the documented 1..27 skater-trait range.".to_string()
                } else {
                    "Goalie trait vocabulary is largely undocumented — only values 1–2 have proposed names.".to_string()
                }),
            ),
        };
        out.push(TraitView {
            slot_field: (*slot_field).to_string(),
            slot_index: slot_index as u32,
            value,
            label,
            confidence,
            note,
        });
    }
    out
}

/// The set of trait values currently occupying at least one slot on the record.
fn active_trait_values(ratings: &[FieldView]) -> std::collections::HashSet<i64> {
    TRAIT_SLOT_ORDER
        .iter()
        .filter_map(|slot| {
            ratings
                .iter()
                .find(|f| &f.studio_field == slot)
                .and_then(|f| f.decoded)
        })
        .filter(|v| *v != 0)
        .collect()
}

/// The full kind-correct trait dictionary as toggleable options, each flagged active when the record
/// currently holds it. This is the checkbox menu's data.
fn build_trait_options(ratings: &[FieldView], is_skater: bool) -> Vec<TraitOption> {
    // No `Trait N` slots in this schema (NHL07 gen6 has no trait system) → no trait menu at all,
    // rather than a dictionary of options that can't be stored anywhere.
    if !TRAIT_SLOT_ORDER.iter().any(|slot| ratings.iter().any(|f| &f.studio_field == slot)) {
        return Vec::new();
    }
    let active = active_trait_values(ratings);
    let dict = if is_skater { SKATER_TRAITS } else { GOALIE_TRAITS };
    dict.iter()
        .map(|(value, label, confidence, note)| TraitOption {
            value: *value,
            label: label.to_string(),
            confidence,
            note: Some(note.to_string()),
            active: active.contains(value),
        })
        .collect()
}

/// Plan the cell writes to activate or deactivate a trait `value`, given the record's current slot
/// values `slots` (each `(slot_field_id, current_value)`). Returns:
/// * `Ok(writes)` — the `(slot_field, new_value)` edits to apply (possibly empty = already in the
///   desired state, a no-op);
/// * `Err(())` — activation requested but all 32 slots are occupied (no free slot).
///
/// Activation writes `value` into the first free slot; deactivation zeroes every slot holding it
/// (so a trait accidentally duplicated across slots is fully cleared).
pub(crate) fn plan_trait_toggle(
    slots: &[(String, i64)],
    value: i64,
    active: bool,
) -> Result<Vec<(String, i64)>, ()> {
    if active {
        if slots.iter().any(|(_, v)| *v == value) {
            return Ok(Vec::new()); // already active
        }
        match slots.iter().find(|(_, v)| *v == 0) {
            Some((field, _)) => Ok(vec![(field.clone(), value)]),
            None => Err(()),
        }
    } else {
        Ok(slots
            .iter()
            .filter(|(_, v)| *v == value)
            .map(|(field, _)| (field.clone(), 0))
            .collect())
    }
}
