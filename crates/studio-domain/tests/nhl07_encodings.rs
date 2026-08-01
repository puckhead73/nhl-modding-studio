//! NHL07 (gen6) value encodings + player types, verified in-game by the user and pinned here against
//! the anchor. These differ from the modern titles and are gated on evidence (field shape / data
//! ranges), never the title id, so the verified NHL 10→21 behaviour is untouched.

use std::path::{Path, PathBuf};

use studio_domain::{player_view, FieldView, PlayerView};
use tdb_core::TdbFile;
use tdb_relations::{PlayerKind, RosterModel};
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;

fn nat_display(v: &PlayerView) -> String {
    v.all_fields.iter().find(|f| f.studio_field == "Lelh").map(|f| f.display.clone()).unwrap_or_default()
}

fn origin_display(v: &PlayerView) -> String {
    v.all_fields.iter().find(|f| f.studio_field == "QvxI").map(|f| f.display.clone()).unwrap_or_default()
}

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn fixtures() -> (Vec<u8>, FieldMetaIndex, EnumRegistry) {
    let bytes = std::fs::read(repo("anchors/xbox360/nhl07/databases/NHL07NG.db")).unwrap();
    let meta = FieldMetaIndex::from_xml(
        &std::fs::read_to_string(repo("anchors/xbox360/nhl07/databases/NHL07NG-meta.xml")).unwrap(),
    )
    .unwrap();
    let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
    (bytes, meta, enums)
}

fn last_name(db: &TdbFile, bio: u32) -> String {
    let t = db.table_canonical("ubPc").unwrap();
    let f = t.fields.iter().find(|f| f.id.canonical(db.endian) == "QbMR").unwrap();
    db.read_inline_str(t, bio, f)
        .map(|b| String::from_utf8_lossy(b).trim_end_matches('\0').to_string())
        .unwrap_or_default()
}

/// Load the full player view for a player found by last name.
fn view_of(name: &str) -> (Vec<u8>, PlayerView) {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let p = model
        .players
        .iter()
        .find(|p| last_name(&db, p.bio_record).eq_ignore_ascii_case(name))
        .unwrap_or_else(|| panic!("{name} in NHL07 roster"));
    let v = player_view(&db, &meta, &enums, &model, p.player_id).unwrap();
    drop(model);
    (bytes, v)
}

fn attr<'a>(v: &'a PlayerView, label: &str) -> Option<&'a FieldView> {
    v.attributes.flat.iter().find(|a| a.label == label).and_then(|a| a.field.as_ref())
}

/// Ratings display on the 50..99 gen6 scale (stored + 50), not the modern +36 — verified: our tool
/// was 14 low vs the game, and the anchor's ratings max at 49 (→ 99).
#[test]
fn ratings_use_the_gen6_plus_50_scale() {
    let (_b, v) = view_of("Weight"); // Doug Weight, a Playmaker
    let passing = attr(&v, "Passing").expect("Passing rating present");
    let decoded = passing.decoded.unwrap();
    assert_eq!(
        passing.display,
        (decoded + 50).to_string(),
        "gen6 ratings show stored + 50 (50..99 scale)"
    );
    assert_eq!(passing.edit_offset, Some(50), "friendly edit stages input - 50");
}

/// Weight is `raw + 150` lb on gen6 (150..275), not the modern `+120`. DiPenta raw 54 → 204 lb.
#[test]
fn weight_uses_the_gen6_plus_150_base() {
    let (_b, v) = view_of("DiPenta");
    assert_eq!(v.header.weight.as_deref(), Some("204 lb"));
    // Height keeps the +54 base on every title: DiPenta raw 20 → 74" = 6'2".
    assert_eq!(v.header.height.as_deref(), Some("6'2\""));
}

/// Skater player types: the 3-bit `p_style` renders the gen6 vocabulary, position-restricted, and a
/// known player resolves to the right archetype. Doug Weight = Playmaker.
#[test]
fn skater_player_type_vocab_and_value() {
    let (_b, v) = view_of("Weight");
    let style = v.attributes.style.as_ref().and_then(|s| s.field.as_ref()).expect("style field");
    assert_eq!(style.enum_label.as_deref(), Some("Playmaker"), "Weight is a Playmaker");
    // A forward only sees the 3..=7 forward roles (plus its own value); no defenseman roles.
    let labels: Vec<&str> = style.enum_options.iter().map(|(_, l)| l.as_str()).collect();
    assert!(labels.contains(&"Playmaker") && labels.contains(&"Sniper") && labels.contains(&"Grinder"));
    assert!(!labels.contains(&"Defensive"), "a forward's dropdown excludes defenseman roles");
    assert_eq!(v.header.style_label.as_deref(), Some("Playmaker"));
}

/// A defenseman only sees the two defenseman roles (Defensive / Offensive). Joe DiPenta = Defensive.
#[test]
fn defenseman_player_type_is_restricted() {
    let (_b, v) = view_of("DiPenta");
    let style = v.attributes.style.as_ref().and_then(|s| s.field.as_ref()).expect("style field");
    assert_eq!(style.enum_label.as_deref(), Some("Defensive"));
    let labels: Vec<&str> = style.enum_options.iter().map(|(_, l)| l.as_str()).collect();
    assert!(labels.contains(&"Defensive") && labels.contains(&"Offensive"));
    assert!(!labels.contains(&"Sniper"), "a defenseman's dropdown excludes forward roles");
}

/// Goalie player types: Bryzgalov = Butterfly (g_style raw 2).
#[test]
fn goalie_player_type_value() {
    let (_b, v) = view_of("Bryzgalov");
    let style = v.attributes.style.as_ref().and_then(|s| s.field.as_ref()).expect("goalie style");
    assert_eq!(style.enum_label.as_deref(), Some("Butterfly"));
    assert_eq!(v.header.style_label.as_deref(), Some("Butterfly"));
}

/// NHL07 uses its own nationality code table — the modern vocabulary mislabels its European players
/// (code 75 = Russia, not the modern "Lithuania"). The gen6 override fixes the label; North-American
/// players are unaffected. And the enum-vocabulary audit no longer floods on the gen6 geo/style codes.
#[test]
fn nationality_uses_gen6_country_table_and_audit_is_clean() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let view = |name: &str| {
        let p = model
            .players
            .iter()
            .find(|p| last_name(&db, p.bio_record).eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{name} in roster"));
        player_view(&db, &meta, &enums, &model, p.player_id).unwrap()
    };
    assert_eq!(nat_display(&view("Afinogenov")), "Russia", "gen6 code 75 = Russia, not Lithuania");
    assert_eq!(nat_display(&view("Bartecko")), "Slovakia");
    assert_eq!(nat_display(&view("Sakic")), "Canada", "North-American codes unchanged");
    assert_eq!(nat_display(&view("Alfredsson")), "Sweden");

    // The gen6 nationality/birth-state/player-style codes no longer flood the audit.
    let findings = tdb_domain_validation::enum_vocabulary(&db, &enums);
    let geo: Vec<_> = findings
        .iter()
        .filter(|f| matches!((f.table.as_str(), f.field.as_str()), ("ubPc", "Lelh") | ("ubPc", "QvxI") | ("eGlu", "QgFs")))
        .collect();
    assert!(geo.is_empty(), "no gen6 geo/style enum-vocabulary findings, got {}", geo.len());
}

/// The "Origin" (birth-state) field resolves NHL07's own nation range, pinned by well-known players'
/// documented birthplaces — including birthplaces distinct from the player's nationality.
#[test]
fn origin_birthplace_codes_resolve() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let view = |name: &str| {
        let p = model
            .players
            .iter()
            .find(|p| last_name(&db, p.bio_record).eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{name} in roster"));
        player_view(&db, &meta, &enums, &model, p.player_id).unwrap()
    };
    assert_eq!(origin_display(&view("Nolan")), "Northern Ireland", "Owen Nolan born in Belfast");
    assert_eq!(origin_display(&view("Regehr")), "Brazil", "Robyn Regehr born in Recife");
    assert_eq!(origin_display(&view("Kolzig")), "South Africa", "Kölzig born in Johannesburg");
    assert_eq!(origin_display(&view("Park")), "South Korea", "Richard Park born in Seoul");
    assert_eq!(origin_display(&view("Kasparaitis")), "Lithuania");
}

/// NHL07 exposes no AI biases and no traits on the Attributes tab.
#[test]
fn no_biases_or_traits_exposed() {
    let (_b, v) = view_of("Weight");
    for bias in ["Shoot / Pass Bias", "Defense / Offense Bias", "Pass / Carry Bias"] {
        assert!(attr(&v, bias).is_none(), "gen6 hides the '{bias}' AI slider");
    }
    assert!(v.attributes.traits.is_empty(), "NHL07 has no traits system");
    assert!(v.attributes.trait_options.is_empty(), "NHL07 has no trait vocabulary");
    // The renamed gen6 ratings surface with their in-game names.
    assert!(attr(&v, "Injury Resist").is_some());
    assert!(attr(&v, "Penalty Resist").is_some());
    assert!(attr(&v, "Intensity").is_some());
}
