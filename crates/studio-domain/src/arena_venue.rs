//! Phase 2.5 — the arena table's **venue parameters**, inventoried per title.
//!
//! The 2.5 matrix row read "venue parameters beyond identity — not inventoried in Studio terms".
//! Research 2026-09-21 pm handoff R51 (`research-r51-arena-venue-inventory`) inventoried all 159
//! distinct fields across the 27 anchors and gave each a **name-based candidate class**. That class
//! map is transcribed into the developer-owned [`defs/arena/venue-fields.toml`]; this module joins
//! it to the *open* database.
//!
//! Two things are deliberately **computed from the open database** rather than taken from the
//! research data, so a title, patch or modified file outside the anchor set still reports the
//! truth:
//!
//! * which fields this title's arena table actually carries, and
//! * whether a field is **constant across every arena row** of this file (a field that never varies
//!   is shown as such and is not offered as an ordinary edit).
//!
//! ## What a class is not
//!
//! A class is a grouping candidate from the field's name and value space. No arena field has a
//! traced runtime consumer (handoff R51 §3). Studio therefore:
//!
//! * shows the class as a label, never as a promise about what the game does;
//! * marks colour-zone fields so the UI can render a swatch (their values are 24-bit RGB — R51 §2);
//! * **refuses to edit** a field whose class is `unknown`, and says why.
//!
//! Nothing here opens a runtime card, because nothing here changes what the game is asked to do.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{DomainError, Result};

/// The candidate class of a venue field, as transcribed from handoff R51.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VenueClass {
    Identity,
    Reference,
    Capacity,
    Audio,
    Horn,
    Lighting,
    Presentation,
    VenueGeometry,
    Flag,
    Unknown,
}

impl VenueClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Reference => "reference",
            Self::Capacity => "capacity",
            Self::Audio => "audio",
            Self::Horn => "horn",
            Self::Lighting => "lighting",
            Self::Presentation => "presentation",
            Self::VenueGeometry => "venue_geometry",
            Self::Flag => "flag",
            Self::Unknown => "unknown",
        }
    }

    /// A one-line, human-readable gloss — the text the card shows beside the group.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Identity => "the arena's own name, id and index",
            Self::Reference => "points at another table's row",
            Self::Capacity => "attendance and crowd figures",
            Self::Audio => "an audio id (arena, city, chant)",
            Self::Horn => "goal / period horn selection and the horn-builder columns",
            Self::Lighting => "goal light, spotlights and netting colour",
            Self::Presentation => "scoreboard, jumbotron, logos and songs",
            Self::VenueGeometry => "seats, walls, tunnels, roof, railings, stairs",
            Self::Flag => "a two-valued switch",
            Self::Unknown => "not classified — neither the name nor the value space suggests a meaning",
        }
    }

    /// Can Studio offer an ordinary cell edit for a field of this class? `unknown` cannot: there is
    /// no basis for saying what a changed value would mean.
    pub fn editable(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

/// One row of the developer-owned class map.
#[derive(Debug, Clone, Deserialize)]
struct MapEntry {
    name: String,
    class: VenueClass,
    #[serde(default)]
    colour: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct MapFile {
    #[serde(default)]
    field: Vec<MapEntry>,
}

/// The class map, loaded from `defs/arena/venue-fields.toml`.
#[derive(Debug, Clone, Default)]
pub struct VenueFieldMap {
    by_name: BTreeMap<String, (VenueClass, bool)>,
}

impl VenueFieldMap {
    /// Load from a `defs` root. A missing file is an error, not an empty map: silently classifying
    /// every field as unknown would look like a measured result.
    pub fn load(defs_root: &Path) -> Result<Self> {
        let path = defs_root.join("arena").join("venue-fields.toml");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| DomainError::Validation(format!("{}: {e}", path.display())))?;
        let parsed: MapFile = toml::from_str(&text)
            .map_err(|e| DomainError::Validation(format!("{}: {e}", path.display())))?;
        let mut by_name = BTreeMap::new();
        for f in parsed.field {
            by_name.insert(f.name, (f.class, f.colour));
        }
        Ok(Self { by_name })
    }

    /// The class of a field, and whether it renders as a colour. An unmapped field is `Unknown` —
    /// a title carrying a field the inventory has never seen is reported, not hidden.
    pub fn classify(&self, name: &str) -> (VenueClass, bool) {
        self.by_name.get(name).copied().unwrap_or((VenueClass::Unknown, false))
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// One venue field of the open title, with everything the card needs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VenueFieldView {
    /// Studio field name (the metadata id).
    pub name: String,
    /// The metadata display name — what the user reads.
    pub label: String,
    pub class: String,
    /// The class gloss, so the card never has to hard-code it.
    pub class_note: String,
    /// This row's raw value.
    pub value: i64,
    /// `value` rendered as `#RRGGBB` when the field is a 24-bit colour zone.
    pub colour: Option<String>,
    /// How many distinct values this field takes across **every arena row of this file**.
    pub distinct: u32,
    /// `Some(v)` when every arena row carries `v` — the field is constant on this title.
    pub constant: Option<i64>,
    /// What this value *names*, when a catalogue resolves it — e.g. a PS4 `goalhorn` value
    /// resolved against `goalhorns.csv` (research 2026-09-21 pm handoff R50). `None` when the
    /// title has no catalogue for the field, or the value names no row; `resolved_note` then says
    /// why. Naming a row is **not** a claim about what the console plays.
    pub resolved: Option<String>,
    /// The rule that resolved it, or the reason nothing did — shown behind disclosure.
    pub resolved_note: Option<String>,
    /// The dictionary key the resolved row names (`$NHLTeamName_ANA`), kept raw beside the text.
    pub resolved_key: Option<String>,
    /// How that key resolved through this project's dictionary: `exact` / `by-hash` / `absent` /
    /// `no-dictionary` (see [`crate::text_keys::KeyLookup`]).
    pub resolved_key_lookup: Option<String>,
    /// Gen7 goal / period horn only (26 Sep 2026): the numbered `.abk` the value names, when the
    /// bank was found and the sound-bank browser lists this title — the card links to it. `None`
    /// for chant / audio-id fields (no selecting consumer traced, R97 §1.4) and every unresolved
    /// or refused reading.
    pub bank_file: Option<String>,
    /// `false` for an `unknown`-class field, and for a constant one: neither is offered as an
    /// ordinary edit. `refusal` says which applies.
    pub editable: bool,
    /// Present when `editable` is false: the reason, in the user's words.
    pub refusal: Option<String>,
}

/// The whole inventory for one arena row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArenaVenueView {
    pub arena_id: u64,
    pub record: u32,
    /// Arena rows in this file — the population `distinct` / `constant` were computed over.
    pub rows: u32,
    /// Every venue field of this title's arena table, in class order then name order.
    pub fields: Vec<VenueFieldView>,
    /// Field counts per class, for the card's summary line.
    pub class_counts: Vec<(String, usize)>,
    pub notes: Vec<String>,
}

/// Fields the arena **identity** card already owns; the inventory lists them under `identity` but
/// never offers them for editing here, so one value never has two editors.
const IDENTITY_OWNED: &[&str] = &["index", "arenaid", "arenaname", "cityname", "table"];

/// Build the inventory for `arena_id` from the open database.
///
/// `values` supplies, per field name, this row's value and the values of every arena row (the
/// caller reads them through the session's field access, so this module stays free of the TDB
/// reader). `labels` supplies the metadata display name.
pub fn build(
    arena_id: u64,
    record: u32,
    rows: u32,
    map: &VenueFieldMap,
    values: &BTreeMap<String, (i64, Vec<i64>)>,
    labels: &BTreeMap<String, String>,
    horns: Option<&crate::goalhorn_catalogue::GoalhornCatalogue>,
    dictionary: Option<&crate::leagues::AttachedDictionary>,
    horn_sources: Option<&crate::gen7_horn_sources::Gen7HornSources>,
) -> ArenaVenueView {
    let mut fields: Vec<VenueFieldView> = Vec::with_capacity(values.len());
    for (name, (value, column)) in values {
        let (class, is_colour) = map.classify(name);
        let mut distinct: Vec<i64> = column.clone();
        distinct.sort_unstable();
        distinct.dedup();
        let constant = (distinct.len() == 1).then(|| distinct[0]);
        let identity_owned = IDENTITY_OWNED.contains(&name.as_str());
        let (editable, refusal) = if !class.editable() {
            (false, Some("this field is not classified: neither its name nor its value space suggests what it means, so Studio does not offer it as an edit (research 2026-09-21 pm handoff R51 inventoried it without a semantic)".to_string()))
        } else if identity_owned {
            (false, Some("edited on the arena's identity card, not here".to_string()))
        } else if let Some(v) = constant {
            (false, Some(format!("constant on this title: every one of the {rows} arena rows carries {v}, so a per-arena edit has no measured meaning")))
        } else {
            (true, None)
        };
        // The one catalogue join Studio has for this table today (R50, PS4 NHL 17–21): the goal
        // horn. `periodhorn` shares the id space but the research did not join it, so it is left
        // as a number rather than resolved by assumption.
        // The row's name is a dictionary key; it resolves through this project's own attached
        // dictionary (item 2 of the coordinator review of receipts 10–17) and stays the raw key,
        // explicitly unresolved, when the dictionary lacks it or none is attached.
        let (resolved, resolved_note, resolved_key, resolved_key_lookup) = match (horns, name.as_str()) {
            (Some(cat), "goalhorn") => {
                let r = cat.resolve(*value);
                match r.row {
                    Some(row) => {
                        let k = crate::text_keys::resolve_key_text(dictionary, &row.name);
                        let key_note = match k.lookup {
                            crate::text_keys::KeyLookup::Exact => format!("name key `{}` resolved through this project's dictionary", row.name),
                            crate::text_keys::KeyLookup::ByHash => format!("name key `{}` resolved by hash through this project's dictionary (row `{}`)", row.name, k.row_key.clone().unwrap_or_default()),
                            crate::text_keys::KeyLookup::Absent => format!("name key `{}` is not in this project's attached dictionary: shown as the key", row.name),
                            crate::text_keys::KeyLookup::NoDictionary => format!("name key `{}` shown as the key: no dictionary is attached to this project", row.name),
                            crate::text_keys::KeyLookup::NotAKey => format!("`{}` is not a dictionary key", row.name),
                        };
                        (
                            Some(format!("{} ({})", k.display(), row.identifier)),
                            Some(format!("{} — {} row; {key_note}; consumer: `sfx_ptc.big` → `goalhornpatch.ptc` (Harmony PTCH patch, research R85 §3 — located, not decoded: the id → element / sample resolution sits inside it)", r.rule, row.class.as_str())),
                            Some(row.name.clone()),
                            Some(k.lookup.as_str().to_string()),
                        )
                    }
                    None => (None, r.refusal.or(Some(r.rule)), None, None),
                }
            }
            // PS4 `periodhorn`: research R85 §3 read the period-horn patch's `PeriodHornID` enum on NHL 21
            // (`Generic` / `Siren` / `Buzzer`); the value is presented as that enum candidate, never as a bank id.
            (Some(_), "periodhorn") => (
                None,
                Some(match *value {
                    0 => "PS4: candidate reading `Generic` — `PeriodHornID` enum of `periodlhornpatch.ptc` (research R85 §3, read on NHL 21; other PS4 titles unmeasured); not a bank id".to_string(),
                    1 => "PS4: candidate reading `Siren` — `PeriodHornID` enum of `periodlhornpatch.ptc` (research R85 §3, read on NHL 21; other PS4 titles unmeasured); not a bank id".to_string(),
                    2 => "PS4: candidate reading `Buzzer` — `PeriodHornID` enum of `periodlhornpatch.ptc` (research R85 §3, read on NHL 21; other PS4 titles unmeasured); not a bank id".to_string(),
                    v => format!("PS4: value {v} is outside the `PeriodHornID` enum read on NHL 21 (0–2, research R85 §3); meaning unknown"),
                }),
                None,
                None,
            ),
            // Gen7 (P4.4, 24 Sep 2026): the horn / chant / audio-id columns read against the
            // install's numbered `audio/sfx` banks under the R78 / R79 / R81 rules — a file by exact
            // presence, a `nocache.big` member by name, a chant *candidate*, no binding for the
            // city / arena ids, and the inspected-sources residual text — never "no horn".
            (None, f) => match horn_sources.and_then(|h| h.resolve(f, *value)) {
                Some((resolved, note)) => (resolved, Some(note), None, None),
                None => (None, None, None, None),
            },
            _ => (None, None, None, None),
        };
        fields.push(VenueFieldView {
            name: name.clone(),
            label: labels.get(name).cloned().unwrap_or_else(|| name.clone()),
            class: class.as_str().to_string(),
            class_note: class.describe().to_string(),
            value: *value,
            colour: (is_colour && *value != 0).then(|| format!("#{:06X}", (*value as u64) & 0xFF_FFFF)),
            distinct: distinct.len() as u32,
            constant,
            resolved,
            resolved_note,
            resolved_key,
            resolved_key_lookup,
            bank_file: if horns.is_none() { horn_sources.and_then(|h| h.bank_file(name, *value)) } else { None },
            editable,
            refusal,
        });
    }
    // Class order first (identity … unknown), then name.
    const ORDER: &[&str] = &["identity", "reference", "capacity", "audio", "horn", "lighting", "presentation", "venue_geometry", "flag", "unknown"];
    let rank = |c: &str| ORDER.iter().position(|o| *o == c).unwrap_or(ORDER.len());
    fields.sort_by(|a, b| rank(&a.class).cmp(&rank(&b.class)).then_with(|| a.name.cmp(&b.name)));

    let mut class_counts: Vec<(String, usize)> = Vec::new();
    for c in ORDER {
        let n = fields.iter().filter(|f| f.class == *c).count();
        if n > 0 {
            class_counts.push(((*c).to_string(), n));
        }
    }
    let colour_fields = fields.iter().filter(|f| map.classify(&f.name).1).count();
    let colours = fields.iter().filter(|f| f.colour.is_some()).count();
    let constants = fields.iter().filter(|f| f.constant.is_some()).count();
    let unknowns = fields.iter().filter(|f| f.class == "unknown").count();
    let mut notes = vec![format!(
        "{} venue fields on this title; {constants} are constant across all {rows} arena rows, {unknowns} are unclassified and not editable, {colours} carry a colour on this row",
        fields.len()
    )];
    if colour_fields > 0 {
        notes.push(format!(
            "{colour_fields} of these are colour zones (24-bit RGB, measured): the card groups and renders them as colours regardless of their class, because R51's class is name-based — `seat*zone*` lands in `capacity` for the word \"seat\" while carrying a colour"
        ));
    }
    notes.push(
        "the class is a candidate from the field's name and value space (research 2026-09-21 pm handoff R51), not a traced runtime effect: no arena field here has a known consumer, and editing one is not a claim that the game reads it"
            .to_string(),
    );
    ArenaVenueView { arena_id, record, rows, fields, class_counts, notes }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> VenueFieldMap {
        VenueFieldMap::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../defs").as_path()).expect("the shipped class map loads")
    }

    /// The developer-owned map covers the research inventory and classifies the fields the handoff
    /// names, including the colour zones.
    #[test]
    fn the_shipped_class_map_covers_the_inventory() {
        let m = map();
        assert_eq!(m.len(), 159, "handoff R51 inventoried 159 distinct fields");
        assert_eq!(m.classify("arenaname").0, VenueClass::Identity);
        assert_eq!(m.classify("maxattendance").0, VenueClass::Capacity);
        assert_eq!(m.classify("goalhorn").0, VenueClass::Horn);
        assert_eq!(m.classify("goallightid").0, VenueClass::Lighting);
        // R51's class is name-based: `seat1zone1` lands in `capacity` because of the word "seat",
        // even though its measured value space is 24-bit RGB. The class is transcribed as the
        // research recorded it (not silently re-grouped); the **measured** colour flag is what the
        // card groups and renders by. Reported back to the research lane as an observation.
        assert_eq!(m.classify("seat1zone1"), (VenueClass::Capacity, true), "the research class is kept; the colour flag is measured");
        assert_eq!(m.classify("wallzone1"), (VenueClass::VenueGeometry, true));
        assert_eq!(m.classify("isoutdoor").0, VenueClass::Flag);
        // A field no inventory has seen is reported as unknown, not dropped.
        assert_eq!(m.classify("a_field_that_does_not_exist"), (VenueClass::Unknown, false));
        assert!(!VenueClass::Unknown.editable());
        assert!(VenueClass::VenueGeometry.editable());
    }

    /// The view: classes sort in order, a constant column refuses with the row count, an unknown
    /// field refuses with the inventory reason, a colour renders as `#RRGGBB`, and identity fields
    /// defer to the identity card.
    /// The one catalogue join the card carries: a PS4 `goalhorn` value reads as the club it names,
    /// with the rule behind disclosure; a value the catalogue cannot place says why instead of
    /// inventing a row; and `periodhorn`, which the research did not join, stays a number.
    #[test]
    fn the_goalhorn_value_resolves_through_the_catalogue_and_periodhorn_does_not() {
        let m = map();
        let cat = crate::goalhorn_catalogue::GoalhornCatalogue::parse(
            "index,identifier,artid,audioid,name,ispremade,ishornelement,isflare,tone
             0,AnaheimDucks,0,0,$NHLTeamName_ANA,1,0,0,-1
             40,GenericHorn1,31,41,$TXT_HORN_ONE,0,0,0,-1
",
            "test",
        )
        .unwrap();
        let mut values: BTreeMap<String, (i64, Vec<i64>)> = BTreeMap::new();
        values.insert("goalhorn".into(), (1, vec![1, 41, 690]));
        values.insert("periodhorn".into(), (1, vec![1, 2, 3]));
        let v = build(9, 1, 3, &m, &values, &BTreeMap::new(), Some(&cat), None, None);
        let by = |n: &str| v.fields.iter().find(|f| f.name == n).unwrap().clone();

        let horn = by("goalhorn");
        assert_eq!(horn.resolved.as_deref(), Some("$NHLTeamName_ANA (AnaheimDucks)"));
        assert!(horn.resolved_note.as_deref().is_some_and(|n| n.contains("value − 1") && n.contains("team") && n.contains("no dictionary is attached")), "{:?}", horn.resolved_note);
        assert_eq!((horn.resolved_key.as_deref(), horn.resolved_key_lookup.as_deref()), (Some("$NHLTeamName_ANA"), Some("no-dictionary")));

        // With this project's own dictionary attached (the committed NHL 17 anchor), the key reads
        // as the club's text and the key stays disclosed; a key the dictionary lacks stays a key.
        let dict_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join("anchors/ps4/nhl17/databases/eng_us.db");
        let dict = crate::leagues::AttachedDictionary::load(&dict_path.to_string_lossy()).unwrap();
        let v = build(9, 1, 3, &m, &values, &BTreeMap::new(), Some(&cat), Some(&dict), None);
        let horn = v.fields.iter().find(|f| f.name == "goalhorn").unwrap();
        assert_eq!(horn.resolved.as_deref(), Some("Anaheim Ducks® (AnaheimDucks)"));
        assert_eq!((horn.resolved_key.as_deref(), horn.resolved_key_lookup.as_deref()), (Some("$NHLTeamName_ANA"), Some("exact")));
        let mut generic: BTreeMap<String, (i64, Vec<i64>)> = BTreeMap::new();
        generic.insert("goalhorn".into(), (41, vec![1, 41, 690]));
        let v = build(9, 1, 3, &m, &generic, &BTreeMap::new(), Some(&cat), Some(&dict), None);
        let horn = v.fields.iter().find(|f| f.name == "goalhorn").unwrap();
        assert_eq!(horn.resolved.as_deref(), Some("$TXT_HORN_ONE (GenericHorn1)"), "an absent key stays the raw key");
        assert_eq!(horn.resolved_key_lookup.as_deref(), Some("absent"));
        assert!(horn.resolved_note.as_deref().unwrap().contains("not in this project's attached dictionary"));

        // `periodhorn` shares the id space but was not joined by the research — left as a number;
        // the note carries the NHL 21 `PeriodHornID` enum reading (R85 §3) as a candidate only.
        assert_eq!(by("periodhorn").resolved, None);
        assert!(by("periodhorn").resolved_note.as_deref().is_some_and(|n| n.contains("`Siren`") && n.contains("not a bank id")), "{:?}", by("periodhorn").resolved_note);
        assert!(horn.resolved_note.as_deref().unwrap().contains("goalhornpatch.ptc"));

        // Without a catalogue nothing resolves, and nothing pretends to.
        let plain = build(9, 1, 3, &m, &values, &BTreeMap::new(), None, None, None);
        assert_eq!(plain.fields.iter().find(|f| f.name == "goalhorn").unwrap().resolved, None);

        // The sentinel: no row, and the reason is carried.
        let mut sentinel: BTreeMap<String, (i64, Vec<i64>)> = BTreeMap::new();
        sentinel.insert("goalhorn".into(), (690, vec![690, 1, 41]));
        let s = build(9, 1, 3, &m, &sentinel, &BTreeMap::new(), Some(&cat), None, None);
        let f = s.fields.iter().find(|f| f.name == "goalhorn").unwrap();
        assert_eq!(f.resolved, None);
        assert!(f.resolved_note.as_deref().is_some_and(|n| n.contains("sentinel")), "{:?}", f.resolved_note);
    }

    #[test]
    fn the_view_marks_constants_colours_and_unclassified_fields() {
        let m = map();
        let mut values: BTreeMap<String, (i64, Vec<i64>)> = BTreeMap::new();
        values.insert("arenaname".into(), (7, vec![7, 8, 9]));
        values.insert("maxattendance".into(), (18_500, vec![18_500, 19_000, 21_000]));
        values.insert("seat1zone1".into(), (0xDF_DFD7, vec![0, 0xDF_DFD7, 0]));
        values.insert("wallzone5".into(), (0, vec![0, 0, 0]));
        values.insert("architectureid".into(), (0, vec![0, 0, 0]));
        values.insert("brand_new_field".into(), (3, vec![1, 2, 3]));
        let mut labels = BTreeMap::new();
        labels.insert("maxattendance".to_string(), "Max attendance".to_string());
        let v = build(12, 5, 3, &m, &values, &labels, None, None, None);

        assert_eq!(v.fields.first().unwrap().name, "arenaname", "identity sorts first");
        let by = |n: &str| v.fields.iter().find(|f| f.name == n).unwrap().clone();

        let name = by("arenaname");
        assert!(!name.editable);
        assert!(name.refusal.unwrap().contains("identity card"));

        let cap = by("maxattendance");
        assert!(cap.editable && cap.refusal.is_none());
        assert_eq!(cap.label, "Max attendance");
        assert_eq!((cap.distinct, cap.constant), (3, None));

        let seat = by("seat1zone1");
        assert_eq!(seat.colour.as_deref(), Some("#DFDFD7"));
        assert!(seat.editable);

        // A zone that is zero everywhere: constant, no swatch, refused with the row count.
        let wall = by("wallzone5");
        assert_eq!(wall.colour, None);
        assert_eq!(wall.constant, Some(0));
        assert!(!wall.editable);
        assert!(wall.refusal.unwrap().contains("all 3 arena rows") || by("architectureid").refusal.is_some());

        // Unclassified: refused for the inventory reason, which outranks "constant".
        let unk = by("brand_new_field");
        assert_eq!(unk.class, "unknown");
        assert!(!unk.editable);
        assert!(unk.refusal.unwrap().contains("not classified"));

        assert!(v.notes.iter().any(|n| n.contains("not a traced runtime effect")));
        assert!(v.class_counts.iter().any(|(c, _)| c == "venue_geometry"));
    }
}
