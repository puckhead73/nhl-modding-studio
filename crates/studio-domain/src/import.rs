//! Phase 5.1 — `EaSlugIndex` / `match_player`: identity-match the open EA project's players by
//! normalized **name + DOB**, the first half of the Roster Builder import bridge.
//!
//! The index folds EA bios with the **same** `tdb_fhm::slug` normalization the FHM corpus uses, so a
//! person matched in Roster Builder and in the corpus resolve identically (Spike S2). It is keyed two
//! ways — name+DOB (the high-confidence primary) and name-only (the fallback for DOB-less bios) — and
//! `match_player` returns **ambiguity-aware candidates**, never a single id: S2 proved even name+DOB
//! is not unique on the EA side (genuine multi-pool duplicate bios), so the cart must decide Match vs
//! Create per item.
//!
//! DOB is composed from `ubPc` `qFnd`/`JKLp`/`Kswi` with the **probed** year base + month/day index
//! (nhl10 is 1-indexed; 11→21 are 0-indexed), read off each bio's own `edit_offset` so this stays the
//! exact inverse of the display path. A year outside the plausibility window is treated as *no usable
//! DOB* (a raw-zero bio composes to ~1900), degrading that bio to the name-only key — the same gate
//! S2 measured with.

use std::collections::{HashMap, HashSet};

use tdb_core::{FieldDescriptor, Table, TdbFile};
use tdb_defs::FieldMetaIndex;
use tdb_edit::txn::{ImportCell, ImportOp, ImportTarget, ImportWrite};
use tdb_enums::EnumRegistry;
use tdb_fhm::slug::{normalize_name, person_slug};
use tdb_relations::entity::ids;
use tdb_relations::RosterModel;

use crate::view::{read_field, team_summaries};

/// Plausible EA birth-year window (mirrors Spike S2). Outside it → no usable DOB → name-only.
const DOB_MIN_YEAR: i64 = 1940;
const DOB_MAX_YEAR: i64 = 2010;

/// How solid a candidate match is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchConfidence {
    /// Matched on normalized name **and** a plausible DOB (high confidence).
    NameAndDob,
    /// Same normalized name, DOB absent or different — a review candidate.
    NameOnly,
}

/// One EA player proposed as the identity match for an incoming (corpus) person. Human-readable
/// surfaces only: names resolve; the raw `ea_player_id` (`wBIz`) rides along for power users.
#[derive(Clone, Debug)]
pub struct MatchCandidate {
    /// The EA player's `ubPc.wBIz`.
    pub ea_player_id: u64,
    pub full_name: String,
    /// Composed `YYYY-MM-DD`, or `None` when the EA bio carries no usable DOB.
    pub dob: Option<String>,
    /// The player's current team name, when resolved.
    pub team_name: Option<String>,
    pub confidence: MatchConfidence,
    /// True when the returned candidate set has more than one entry — the cart must disambiguate.
    pub ambiguous: bool,
}

/// One indexed EA bio row (the data a candidate is rendered from).
#[derive(Clone, Debug)]
struct BioRow {
    full_name: String,
    dob: Option<String>,
    team_name: Option<String>,
}

/// A lazily-built, reusable index over the open EA project's players. Build **once** and reuse across
/// cart items (caching is the service layer's job — Phase 5.4); never rebuild per lookup.
pub struct EaSlugIndex {
    by_name_dob: HashMap<String, Vec<u64>>,
    by_name: HashMap<String, Vec<u64>>,
    rows: HashMap<u64, BioRow>,
}

/// The name-only fold: `normalize_name(last)|normalize_name(first)` — reused verbatim on both sides.
fn name_key(last: &str, first: &str) -> String {
    format!("{}|{}", normalize_name(last), normalize_name(first))
}

/// A composed EA birth year within the plausibility window? (Excludes the ~1900 zero-sentinel.)
fn plausible_year(dob: &str) -> bool {
    dob.split('-')
        .next()
        .and_then(|y| y.trim().parse::<i64>().ok())
        .is_some_and(|y| (DOB_MIN_YEAR..=DOB_MAX_YEAR).contains(&y))
}

/// The first `ubPc` field descriptor whose metadata `name` matches (name-keyed bio fields).
fn descriptor_by_name<'a>(meta: &FieldMetaIndex, table: &'a Table, name: &str) -> Option<&'a FieldDescriptor> {
    table
        .fields
        .iter()
        .find(|f| meta.field(&table.id.raw(), &f.id.raw()).map(|m| m.name == name).unwrap_or(false))
}

impl EaSlugIndex {
    /// Build the index over every player in the roster model. O(players); reads only the five bio
    /// fields it needs (name ×2, birthdate ×3) — deliberately **not** `player_view` per player, which
    /// rebuilds the entity model each call (Spike S2/S3).
    pub fn build(
        db: &TdbFile,
        meta: &FieldMetaIndex,
        enums: &EnumRegistry,
        model: &RosterModel,
    ) -> Self {
        let mut idx = EaSlugIndex {
            by_name_dob: HashMap::new(),
            by_name: HashMap::new(),
            rows: HashMap::new(),
        };
        let Some(bio) = db.table_canonical(ids::BIO) else {
            return idx;
        };

        // Team-name resolver (human-readable surfaces) + each player's current team from the slots.
        let team_names: HashMap<u64, String> =
            team_summaries(db, meta, enums, model).into_iter().map(|t| (t.team_id, t.name)).collect();
        let mut team_of: HashMap<u64, u64> = HashMap::new();
        for s in &model.slots {
            if let Some(pid) = s.player_id {
                team_of.entry(pid).or_insert(s.team_id);
            }
        }

        let first_f = descriptor_by_name(meta, bio, "firstname");
        let last_f = descriptor_by_name(meta, bio, "lastname");
        let by_id = |id: &str| bio.fields.iter().find(|f| f.id.canonical(db.endian) == id);
        let (qfnd, jklp, kswi) = (by_id("qFnd"), by_id("JKLp"), by_id("Kswi"));

        // Probe the birthdate offsets **once** off any real record via the display path's own
        // `edit_offset` (year base 1900/1959; month/day 0- or 1-index). Constant per db.
        let sample = model.players.first().map(|p| p.bio_record);
        let year_base = sample
            .zip(qfnd)
            .and_then(|(rec, f)| read_field(db, meta, enums, bio, rec, f).edit_offset)
            .unwrap_or(1900);
        let md_off = sample
            .zip(jklp)
            .and_then(|(rec, f)| read_field(db, meta, enums, bio, rec, f).edit_offset)
            .unwrap_or(0);

        let read_str = |f: Option<&FieldDescriptor>, rec: u32| -> String {
            f.and_then(|f| db.read_inline_str(bio, rec, f))
                .map(|b| String::from_utf8_lossy(b).trim().to_string())
                .unwrap_or_default()
        };
        let read_dec = |f: Option<&FieldDescriptor>, rec: u32| -> Option<i64> {
            let f = f?;
            let raw = db.read_u64(bio, rec, f)?;
            Some(meta.field(&bio.id.raw(), &f.id.raw()).map(|m| m.decode_raw(raw)).unwrap_or(raw as i64))
        };

        for p in &model.players {
            let rec = p.bio_record;
            let first = read_str(first_f, rec);
            let last = read_str(last_f, rec);
            let full_name = match (first.as_str(), last.as_str()) {
                ("", l) => l.to_string(),
                (f, "") => f.to_string(),
                (f, l) => format!("{f} {l}"),
            };

            // Compose the DOB with the probed offsets — same formula as the display path.
            let dob = match (read_dec(qfnd, rec), read_dec(jklp, rec), read_dec(kswi, rec)) {
                (Some(y), Some(m), Some(d)) => {
                    let (y, m, d) = (y + year_base, m + md_off, d + md_off);
                    if (DOB_MIN_YEAR..=DOB_MAX_YEAR).contains(&y)
                        && (1..=12).contains(&m)
                        && (1..=31).contains(&d)
                    {
                        Some(format!("{y:04}-{m:02}-{d:02}"))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let team_name = team_of.get(&p.player_id).and_then(|t| team_names.get(t)).cloned();

            idx.by_name.entry(name_key(&last, &first)).or_default().push(p.player_id);
            if let Some(d) = &dob {
                idx.by_name_dob.entry(person_slug(&last, &first, d)).or_default().push(p.player_id);
            }
            idx.rows.insert(p.player_id, BioRow { full_name, dob, team_name });
        }
        idx
    }

    /// How many players are indexed.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True when the index holds no players.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Candidate EA players for an incoming person, ranked: **name+DOB** matches first (high
    /// confidence), then any other **same-name** players (review candidates). Empty ⇒ no match ⇒ the
    /// cart should Create. `dob` is the incoming person's `YYYY-MM-DD` (or `None`); an implausible
    /// year degrades to the name-only path, exactly like the index side.
    pub fn match_player(&self, last: &str, first: &str, dob: Option<&str>) -> Vec<MatchCandidate> {
        let mut out: Vec<MatchCandidate> = Vec::new();
        let mut seen: HashSet<u64> = HashSet::new();

        // 1. Exact name + plausible DOB — the high-confidence head of the list.
        if let Some(d) = dob.filter(|d| plausible_year(d)) {
            if let Some(ids) = self.by_name_dob.get(&person_slug(last, first, d)) {
                for &id in ids {
                    if seen.insert(id) {
                        out.push(self.candidate(id, MatchConfidence::NameAndDob));
                    }
                }
            }
        }

        // 2. Any other player sharing the normalized name — review candidates (covers DOB-less or
        // DOB-mismatched bios, and surfaces the multi-pool duplicates S2 found).
        if let Some(ids) = self.by_name.get(&name_key(last, first)) {
            for &id in ids {
                if seen.insert(id) {
                    out.push(self.candidate(id, MatchConfidence::NameOnly));
                }
            }
        }

        // Ambiguity is a property of the returned set: >1 candidate ⇒ the cart must disambiguate.
        let ambiguous = out.len() > 1;
        for c in &mut out {
            c.ambiguous = ambiguous;
        }
        out
    }

    fn candidate(&self, id: u64, confidence: MatchConfidence) -> MatchCandidate {
        let row = self.rows.get(&id);
        MatchCandidate {
            ea_player_id: id,
            full_name: row.map(|r| r.full_name.clone()).unwrap_or_default(),
            dob: row.and_then(|r| r.dob.clone()),
            team_name: row.and_then(|r| r.team_name.clone()),
            confidence,
            ambiguous: false, // set by match_player once the full set is known
        }
    }
}

// ---------------------------------------------------------------------------
// Lowering: corpus person-season → encoded ImportOp (Phase 5.2b)
// ---------------------------------------------------------------------------

/// The bio + ratings for one corpus person-season, resolved into EA-ready **human** values. This is
/// the seam between the corpus read (the adapter that fills it) and [`lower_create`], which encodes it
/// against the target project — so lowering is testable with synthetic input, no corpus required.
#[derive(Clone, Debug, Default)]
pub struct PersonSeason {
    pub first_name: String,
    pub last_name: String,
    /// EA position code 0..4 (4 = goalie). Written by the cluster create, not as a field write.
    pub position: u64,
    pub height_inches: Option<i64>,
    pub weight_lb: Option<i64>,
    /// Absolute `(year, month, day)`.
    pub birthdate: Option<(i64, i64, i64)>,
    pub jersey: Option<i64>,
    /// Handedness enum raw for `ubPc.GRkp` (0 = Left, 1 = Right), already resolved against the target
    /// title's enum vocabulary by the adapter. `None` when the corpus carries no usable hand.
    pub handedness: Option<u64>,
    /// Nationality enum raw for `ubPc.Lelh`, resolved from the corpus country name against the target
    /// title's enum. `None` when the country is absent/unmapped (skip, never guessed).
    pub nationality: Option<u64>,
    /// Skater style enum raw for `dSvy.QgFs` (1-based; the archetype classifier's `ea_raw`). `None`
    /// for goalies (no classifier) or when no attributes were scorable.
    pub style: Option<u64>,
    /// `(companion studio field id, human rating on the display scale)`. The rating name→field
    /// resolution is the corpus adapter's job (`compute_person_ratings` + the rating vocabulary).
    pub ratings: Vec<(String, i64)>,
}

/// A field `lower_create` could not write, and why — surfaced in preview so a dropped value is never
/// silent (S1 skip-and-report; `Capabilities`-style title gaps).
#[derive(Clone, Debug)]
pub struct SkippedField {
    pub field: String,
    pub reason: String,
}

/// The dry-run result of a change set: what commit *would* do, computed without mutating the session
/// (Phase 5.2 two-phase pipeline). Because commit re-runs the identical lowered ops from the same
/// state, `previewed` is byte-for-byte what commit writes — the pipeline's headline invariant.
#[derive(Clone, Debug)]
pub struct ImportPreview {
    /// Player count before / after (the capacity signal; a fuller per-table projection is 5.2b-next).
    pub players_before: usize,
    pub players_after: usize,
    /// The four db-only audit predicates over the *previewed* state, resolved to human-readable text.
    pub audit: crate::audit::HumanReport,
    /// Fields that could not be written (out of range / absent on this title).
    pub skipped: Vec<SkippedField>,
    /// The exact bytes commit would produce (CRCs finalized) — proves preview == commit and lets a
    /// caller hash the outcome.
    pub previewed: Vec<u8>,
}

/// Encode one human value on `table.field` as `human − edit_offset` and push it, or record why it was
/// skipped. `edit_offset` is the field's own **probed** semantic offset (height +54, weight base,
/// birthdate 0/1-index, rating +36/+50/0) read off record 0 — record-independent — so the write is the
/// exact inverse of the display path (Spike S1). Out-of-range → skip-and-report, never a guessed write.
#[allow(clippy::too_many_arguments)]
fn push_semantic_int(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    table: &Table,
    target: ImportTarget,
    writes: &mut Vec<ImportWrite>,
    skipped: &mut Vec<SkippedField>,
    field_id: &str,
    human: i64,
) {
    let Some(f) = table.fields.iter().find(|f| f.id.canonical(db.endian) == field_id) else {
        skipped.push(SkippedField { field: field_id.to_string(), reason: "field absent on this title".into() });
        return;
    };
    let offset = read_field(db, meta, enums, table, 0, f).edit_offset.unwrap_or(0);
    let decoded = human - offset;
    match meta.field(&table.id.raw(), &f.id.raw()) {
        Some(m) if m.validate(decoded).is_ok() => {
            writes.push(ImportWrite {
                target,
                field: field_id.to_string(),
                cell: ImportCell::Int(m.encode(decoded)),
            });
        }
        _ => skipped.push(SkippedField {
            field: field_id.to_string(),
            reason: format!("human value {human} out of range (or field lacks metadata) on this title"),
        }),
    }
}

/// Build the encoded field writes for a person-season against `bio` + the kind-correct `companion`
/// table — shared by the Create and Match lowerings. Each field uses its own probed `edit_offset`;
/// out-of-range or absent fields are skip-and-reported, never guessed.
fn person_writes(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    bio: &Table,
    companion: Option<&Table>,
    person: &PersonSeason,
) -> (Vec<ImportWrite>, Vec<SkippedField>) {
    let mut writes: Vec<ImportWrite> = Vec::new();
    let mut skipped: Vec<SkippedField> = Vec::new();

    // Identity strings (no offset).
    for (name, text) in [("firstname", &person.first_name), ("lastname", &person.last_name)] {
        match descriptor_by_name(meta, bio, name) {
            Some(f) => writes.push(ImportWrite {
                target: ImportTarget::Bio,
                field: f.id.canonical(db.endian),
                cell: ImportCell::Str(text.clone()),
            }),
            None => skipped.push(SkippedField {
                field: name.to_string(),
                reason: "field absent on this title".into(),
            }),
        }
    }

    // Physicals + jersey (each via its own probed edit_offset).
    if let Some(h) = person.height_inches {
        push_semantic_int(db, meta, enums, bio, ImportTarget::Bio, &mut writes, &mut skipped, "ypBQ", h);
    }
    if let Some(w) = person.weight_lb {
        push_semantic_int(db, meta, enums, bio, ImportTarget::Bio, &mut writes, &mut skipped, "sNZW", w);
    }
    if let Some(j) = person.jersey {
        push_semantic_int(db, meta, enums, bio, ImportTarget::Bio, &mut writes, &mut skipped, "sVRt", j);
    }

    // Birthdate — each component decodes with the probed year base / month-day 0-or-1 index.
    if let Some((y, m, d)) = person.birthdate {
        push_semantic_int(db, meta, enums, bio, ImportTarget::Bio, &mut writes, &mut skipped, "qFnd", y);
        push_semantic_int(db, meta, enums, bio, ImportTarget::Bio, &mut writes, &mut skipped, "JKLp", m);
        push_semantic_int(db, meta, enums, bio, ImportTarget::Bio, &mut writes, &mut skipped, "Kswi", d);
    }

    // Enum-coded identity — handedness + nationality on the bio. Each carries a raw enum code the
    // adapter already resolved against the target title's vocabulary (offset 0; validated as a plain
    // int by `push_semantic_int`, and as an enum member by the op audit).
    if let Some(hand) = person.handedness {
        push_semantic_int(db, meta, enums, bio, ImportTarget::Bio, &mut writes, &mut skipped, "GRkp", hand as i64);
    }
    if let Some(nat) = person.nationality {
        push_semantic_int(db, meta, enums, bio, ImportTarget::Bio, &mut writes, &mut skipped, "Lelh", nat as i64);
    }

    // Ratings + skater style on the kind-correct companion (Spike S1 rating edit_offset). Style is the
    // 1-based `QgFs` archetype code (skaters only; the adapter leaves it `None` for goalies).
    if let Some(comp) = companion {
        if let Some(style) = person.style {
            push_semantic_int(db, meta, enums, comp, ImportTarget::Companion, &mut writes, &mut skipped, "QgFs", style as i64);
        }
        for (fid, human) in &person.ratings {
            push_semantic_int(db, meta, enums, comp, ImportTarget::Companion, &mut writes, &mut skipped, fid, *human);
        }
    }

    (writes, skipped)
}

/// Lower one **CreateNew** person-season into an encoded [`ImportOp::Create`] against the open project.
/// Position is written by the cluster create; identity strings, physicals, birthdate, jersey, and
/// ratings become field writes with each target field's own probed offset. **Not yet mapped**
/// (TODO 5.2b-next): enum-coded identity (nationality / handedness / birthplace) and player style.
/// Returns the op + a skip report.
pub fn lower_create(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    person: &PersonSeason,
    target_team: u64,
) -> Option<(ImportOp, Vec<SkippedField>)> {
    let bio = db.table_canonical(ids::BIO)?;
    let companion_id = if person.position == 4 { ids::GOALIE_AI } else { ids::SKATER_AI };
    let (writes, skipped) =
        person_writes(db, meta, enums, bio, db.table_canonical(companion_id), person);
    Some((ImportOp::Create { team: target_team, position: person.position, writes }, skipped))
}

/// Lower one **Match(existing)** person-season into an encoded [`ImportOp::Update`] over the matched
/// EA player's own records — no allocation. The companion table follows the **existing** player's kind
/// (a matched player keeps its cluster shape); position itself is not rewritten. Returns `None` if the
/// player id is unknown.
pub fn lower_update(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    person: &PersonSeason,
    ea_player_id: u64,
) -> Option<(ImportOp, Vec<SkippedField>)> {
    let bio = db.table_canonical(ids::BIO)?;
    let model = RosterModel::build(db).ok()?;
    let player = model.player(ea_player_id)?;
    let companion_id = match player.kind {
        tdb_relations::PlayerKind::Skater => ids::SKATER_AI,
        tdb_relations::PlayerKind::Goalie => ids::GOALIE_AI,
    };
    let (writes, skipped) =
        person_writes(db, meta, enums, bio, db.table_canonical(companion_id), person);
    Some((ImportOp::Update { player_id: ea_player_id, writes }, skipped))
}

// ---------------------------------------------------------------------------
// Corpus adapter: (person_id, season_year) → PersonSeason (Phase 5.2b)
// ---------------------------------------------------------------------------

/// EA player-position code (`ubPc.vjla`) for a corpus `C/LW/RW/D/G` label — mirrors
/// `tdb_relations::Position::from_raw` (0 Center, 1 LW, 2 RW, 3 Defense, 4 Goalie).
fn ea_position_code(label: &str) -> u64 {
    match label {
        "LW" => 1,
        "RW" => 2,
        "D" | "LD" | "RD" => 3,
        "G" => 4,
        _ => 0, // "C" and anything unexpected → Center
    }
}

/// Parse a corpus `dob` (`YYYY-MM-DD`, or `?`/blank) into absolute `(year, month, day)`.
fn parse_dob(dob: &str) -> Option<(i64, i64, i64)> {
    let mut it = dob.trim().split('-');
    match (it.next(), it.next(), it.next()) {
        (Some(y), Some(m), Some(d)) => Some((
            y.trim().parse().ok()?,
            m.trim().parse().ok()?,
            d.trim().parse().ok()?,
        )),
        _ => None,
    }
}

/// Bridge the formula's EA-attribute **display name** (`EaRating.ea_key`) to the on-disk rating-field
/// **suffix** (`rating_suffix(name)`). The two vocabularies mostly agree but diverge on a handful of
/// spellings (`Aggressiveness`→`aggression`, `Fighting Skill`→`fightingskill`), so the mapping is
/// explicit and verified against `attributes::{SKATER,GOALIE}_LABELS`. The three AI **biases** (0..15,
/// id-keyed) and goalie **Angles** (no unambiguous field) are intentionally absent → skip, not guess.
const SKATER_EA_TO_SUFFIX: &[(&str, &str)] = &[
    ("Speed", "speed"), ("Body Checking", "bodychecking"), ("Endurance", "endurance"),
    ("Puck Control", "puckcontrol"), ("Passing", "passing"), ("Slapshot Power", "slapshotpower"),
    ("Slapshot Accuracy", "slapaccuracy"), ("Wristshot Power", "wristshotpower"),
    ("Wristshot Accuracy", "wristaccuracy"), ("Agility", "agility"), ("Strength", "strength"),
    ("Acceleration", "acceleration"), ("Balance", "balance"), ("Faceoffs", "faceoffs"),
    ("Durability", "durability"), ("Deking", "deking"), ("Aggressiveness", "aggression"),
    ("Poise", "poise"), ("Hand-Eye", "handeye"), ("Shot Blocking", "shotblocking"),
    ("Offensive Awareness", "offawareness"), ("Defensive Awareness", "defawareness"),
    ("Discipline", "discipline"), ("Fighting Skill", "fightingskill"), ("Stickchecking", "stickchecking"),
];
const GOALIE_EA_TO_SUFFIX: &[(&str, &str)] = &[
    ("Glove Low", "gsl"), ("Glove High", "gsh"), ("Stick Low", "ssl"), ("Stick High", "ssh"),
    ("Speed", "speed"), ("Agility", "agility"), ("Consistency", "consistency"),
    ("Pokechecking", "pokecheck"), ("Endurance", "endurance"), ("Breakaways", "breakaway"),
    ("Rebounds", "reboundcontrol"), ("Shot Recovery", "shotrecover"), ("Poise", "poise"),
    ("Passing", "passing"), ("Puck Playing Frequency", "puckplayfrequency"),
    ("Aggressiveness", "aggression"), ("Durability", "durability"), ("Vision", "vision"),
];

/// FHM country names that differ from the EA nationality enum label. Only genuine spelling
/// divergences (verified against the corpus vocab) — everything else matches the label verbatim, and
/// an unmapped country resolves to `None` (skip, never guessed).
const NATIONALITY_ALIASES: &[(&str, &str)] = &[("Czechia", "Czech Republic")];

/// Resolve a corpus label to a target-title enum raw code for `table.field`, or `None` when the field
/// carries no enum on this title or the label is not in its vocabulary. The single source of truth is
/// the loaded, verified enum def — so this stays title-aware and never hardcodes a code.
fn resolve_enum(enums: &EnumRegistry, table_canonical: &str, field: &str, label: &str) -> Option<u64> {
    let raw = enums.for_field(table_canonical, field)?.raw_of(label)?;
    u64::try_from(raw).ok()
}

/// Map a computed EA rating set onto the **target** companion table's field ids, via the explicit
/// `ea_key → suffix` bridge, then `suffix → field` per the loaded title. Only standard 0..99 ratings
/// are mapped; biases and unmapped attrs are dropped (deferred, not guessed). Returns
/// `(field_id, human_value)` pairs ready for `PersonSeason.ratings`.
fn resolve_ratings(
    set: &tdb_formula::EaRatingSet,
    db: &TdbFile,
    meta: &FieldMetaIndex,
    goalie: bool,
) -> Vec<(String, i64)> {
    let table_id = if goalie { ids::GOALIE_AI } else { ids::SKATER_AI };
    let Some(comp) = db.table_canonical(table_id) else {
        return Vec::new();
    };
    let bridge = if goalie { GOALIE_EA_TO_SUFFIX } else { SKATER_EA_TO_SUFFIX };

    // suffix → this title's field id, for the companion's rating fields.
    let mut field_of: HashMap<&str, String> = HashMap::new();
    for f in &comp.fields {
        if let Some(m) = meta.field(&comp.id.raw(), &f.id.raw()) {
            if let Some(suffix) = crate::attributes::rating_suffix(&m.name) {
                field_of.insert(suffix, f.id.canonical(db.endian));
            }
        }
    }

    let mut out = Vec::new();
    for r in &set.ratings {
        let Some(v) = r.value else { continue };
        let Some((_, suffix)) = bridge.iter().find(|(k, _)| *k == r.ea_key) else { continue };
        if let Some(fid) = field_of.get(suffix) {
            out.push((fid.clone(), v as i64));
        }
    }
    out
}

/// Build a [`PersonSeason`] for `(person_id, season_year)` from the corpus, with ratings resolved to
/// the **target** project's companion fields (the corpus→EA bridge — the first read that touches both
/// databases). Height/weight come from the season row when present, else the person's latest bio;
/// ratings are computed via [`crate::formula::compute_person_ratings`] under `profile`/`maxima` and
/// mapped by rating key. Jersey is left to the cart (`ImportItem`). Enum-coded identity
/// (nationality/handedness/birthplace) + style are **not yet** mapped. Returns `None` if the person is
/// absent from the corpus.
#[allow(clippy::too_many_arguments)]
pub fn person_season_from_corpus(
    corpus: &crate::historical::FhmCorpus,
    person_id: i64,
    season_year: i64,
    profile: &tdb_formula::FormulaProfile,
    maxima: &tdb_fhm_store::AttributeMaxima,
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
) -> Result<Option<PersonSeason>, crate::FhmError> {
    let Some(detail) = corpus.person_detail(person_id)? else {
        return Ok(None);
    };

    let position = if detail.is_goalie {
        4
    } else {
        detail.position.as_deref().map(ea_position_code).unwrap_or(0)
    };
    let season = detail.seasons.iter().find(|s| s.season_year == season_year);
    let height_inches = season.and_then(|s| s.height).or(detail.height);
    let weight_lb = season.and_then(|s| s.weight).or(detail.weight);
    let birthdate = parse_dob(&detail.dob);

    // Enum-coded identity, resolved against the **target title's** vocabulary (the bio table's own
    // canonical id). Handedness labels ("Left"/"Right") match the enum verbatim; nationality needs the
    // FHM→EA spelling alias. Anything unmapped resolves to `None` — skip, never guessed.
    let bio_canon = db.table_canonical(ids::BIO).map(|t| t.id.canonical(db.endian));
    let handedness = bio_canon.as_deref().zip(detail.hand.as_deref()).and_then(|(tbl, hand)| {
        resolve_enum(enums, tbl, "GRkp", hand.trim())
    });
    let nationality = bio_canon.as_deref().zip(detail.nationality.as_deref()).and_then(|(tbl, country)| {
        let country = country.trim();
        let mapped = NATIONALITY_ALIASES.iter().find(|(k, _)| *k == country).map(|(_, v)| *v).unwrap_or(country);
        resolve_enum(enums, tbl, "Lelh", mapped)
    });

    // Ratings + skater style: one `compute_person_ratings` pass yields both (the archetype's `ea_raw`
    // is the 1-based `QgFs` code; the classifier returns `None` for goalies).
    let (ratings, style) = match crate::formula::compute_person_ratings(corpus, person_id, season_year, profile, maxima)? {
        Some(cr) => {
            let style = cr.set.archetype.as_ref().map(|a| a.ea_raw as u64);
            (resolve_ratings(&cr.set, db, meta, detail.is_goalie), style)
        }
        None => (Vec::new(), None),
    };

    Ok(Some(PersonSeason {
        first_name: detail.first_name,
        last_name: detail.last_name,
        position,
        height_inches,
        weight_lb,
        birthdate,
        jersey: None,
        handedness,
        nationality,
        style,
        ratings,
    }))
}

// ---------------------------------------------------------------------------
// Cart orchestration: ImportChangeSet → lowered ops (Phase 5.2b-next)
// ---------------------------------------------------------------------------

/// The identity a cart item resolves to: create a brand-new EA player, or update one already in the
/// project (matched via [`EaSlugIndex::match_player`]). The two lower to different ops — Create
/// allocates a cluster; Match writes over the existing records with **no allocation**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportIdentity {
    /// Allocate a new player cluster on the item's `target_team_id`.
    CreateNew,
    /// Update the existing EA player with this `wBIz` (no allocation, keeps its current team).
    Match(u64),
}

/// One line of a Roster Builder cart: a corpus person-season aimed at an EA team, with the
/// create-vs-match decision and an optional jersey override. `profile`/`maxima` are supplied to
/// [`lower_change_set`] once for the whole cart (a per-item profile is a Phase 5.4 concern).
#[derive(Clone, Debug)]
pub struct ImportItem {
    pub person_id: i64,
    pub season_year: i64,
    /// The EA team the created player lands on. Advisory for `Match` (which keeps the player's team).
    pub target_team_id: u64,
    pub identity: ImportIdentity,
    /// Overrides the person-season's jersey number when `Some`.
    pub jersey: Option<i64>,
}

/// What lowering did with one cart item — drives the preview's per-item rows. Human-readable
/// surfaces: `full_name` is resolved; raw ids ride along for power users.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemOutcome {
    /// Lowered to an `ImportOp::Create` on `target_team_id`.
    Created,
    /// Lowered to an `ImportOp::Update` over EA player `wBIz`.
    Matched(u64),
    /// The person id is not in the corpus — nothing lowered for this item.
    PersonMissing,
    /// A `Match` item pointed at an EA player id absent from the project — nothing lowered.
    NoSuchPlayer(u64),
    /// The project's bio table is missing (a malformed target) — nothing lowered.
    NoBioTable,
}

/// The per-item trail of a lowered cart: which person/season/target, the resolved name, the outcome,
/// and any fields this item had to skip. One entry per input [`ImportItem`], in cart order.
#[derive(Clone, Debug)]
pub struct LoweredItem {
    pub person_id: i64,
    pub season_year: i64,
    pub target_team_id: u64,
    pub full_name: String,
    pub outcome: ItemOutcome,
    pub skipped: Vec<SkippedField>,
}

impl LoweredItem {
    /// True when this item produced an encoded op (i.e. it will appear in a preview/commit).
    pub fn lowered(&self) -> bool {
        matches!(self.outcome, ItemOutcome::Created | ItemOutcome::Matched(_))
    }
}

/// A whole cart lowered against the open project: the encoded ops (in cart order, only for items that
/// lowered), the merged skip report, and the per-item trail. Preview and commit both consume `ops`,
/// so lowering **once** and feeding the same `ops` to both keeps them byte-identical (the pipeline's
/// headline invariant). Items that could not lower (missing person, unknown match) carry a failing
/// [`ItemOutcome`] and contribute **no** op — a caller gates commit on `blocked()`.
#[derive(Clone, Debug, Default)]
pub struct LoweredChangeSet {
    pub ops: Vec<ImportOp>,
    pub skipped: Vec<SkippedField>,
    pub items: Vec<LoweredItem>,
}

impl LoweredChangeSet {
    /// The items that failed to lower (a blocker: commit should be disabled until the cart is fixed).
    pub fn blocked(&self) -> impl Iterator<Item = &LoweredItem> {
        self.items.iter().filter(|i| !i.lowered())
    }

    /// True when at least one cart item failed to lower.
    pub fn has_blockers(&self) -> bool {
        self.blocked().next().is_some()
    }
}

/// Tally the new player clusters a lowered cart adds, by kind (position 4 = goalie) — the input to the
/// capacity projection. Updates (Match) allocate nothing and are ignored.
pub fn count_creates(ops: &[ImportOp]) -> crate::capacity::PendingCreates {
    let mut c = crate::capacity::PendingCreates::default();
    for op in ops {
        if let ImportOp::Create { position, .. } = op {
            if *position == 4 {
                c.goalies += 1;
            } else {
                c.skaters += 1;
            }
        }
    }
    c
}

/// The display name for a person-season (human-readable surfaces).
fn person_full_name(ps: &PersonSeason) -> String {
    match (ps.first_name.trim(), ps.last_name.trim()) {
        ("", l) => l.to_string(),
        (f, "") => f.to_string(),
        (f, l) => format!("{f} {l}"),
    }
}

/// Lower a whole cart of [`ImportItem`]s into encoded ops against the open project — the single seam
/// the Roster Builder drives. Each item is resolved through the corpus adapter
/// ([`person_season_from_corpus`]) then lowered by [`lower_create`] (CreateNew) or [`lower_update`]
/// (Match); the ops accumulate in cart order and the per-item trail records every outcome and skip.
///
/// One `profile`/`maxima` pair covers the whole cart (a per-item profile is Phase 5.4). Items that
/// cannot lower — person absent from the corpus, a `Match` at an unknown id — do **not** abort the
/// lowering; they carry a failing [`ItemOutcome`] and add no op, so the preview can show them as
/// blockers. Returns the merged [`LoweredChangeSet`] ready for `preview_import`/`commit_import`.
#[allow(clippy::too_many_arguments)]
pub fn lower_change_set(
    corpus: &crate::historical::FhmCorpus,
    items: &[ImportItem],
    profile: &tdb_formula::FormulaProfile,
    maxima: &tdb_fhm_store::AttributeMaxima,
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
) -> Result<LoweredChangeSet, crate::FhmError> {
    let mut out = LoweredChangeSet::default();

    for item in items {
        // Resolve the corpus person-season (applying the cart's jersey override).
        let ps = match person_season_from_corpus(
            corpus, item.person_id, item.season_year, profile, maxima, db, meta, enums,
        )? {
            Some(mut ps) => {
                if item.jersey.is_some() {
                    ps.jersey = item.jersey;
                }
                ps
            }
            None => {
                out.items.push(LoweredItem {
                    person_id: item.person_id,
                    season_year: item.season_year,
                    target_team_id: item.target_team_id,
                    full_name: String::new(),
                    outcome: ItemOutcome::PersonMissing,
                    skipped: Vec::new(),
                });
                continue;
            }
        };
        let full_name = person_full_name(&ps);

        // Lower per the identity decision; record the outcome + merge the item's skips.
        let (outcome, op, skipped) = match item.identity {
            ImportIdentity::CreateNew => match lower_create(db, meta, enums, &ps, item.target_team_id) {
                Some((op, sk)) => (ItemOutcome::Created, Some(op), sk),
                None => (ItemOutcome::NoBioTable, None, Vec::new()),
            },
            ImportIdentity::Match(id) => match lower_update(db, meta, enums, &ps, id) {
                Some((op, sk)) => (ItemOutcome::Matched(id), Some(op), sk),
                None => (ItemOutcome::NoSuchPlayer(id), None, Vec::new()),
            },
        };

        if let Some(op) = op {
            out.ops.push(op);
            out.skipped.extend(skipped.iter().cloned());
        }
        out.items.push(LoweredItem {
            person_id: item.person_id,
            season_year: item.season_year,
            target_team_id: item.target_team_id,
            full_name,
            outcome,
            skipped,
        });
    }

    Ok(out)
}
