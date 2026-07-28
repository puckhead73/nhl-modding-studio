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

/// Lower one **CreateNew** person-season into an encoded [`ImportOp::Create`] against the open project.
/// Position is written by the cluster create; identity strings, physicals, birthdate, jersey, and
/// ratings become field writes with each target field's own probed offset. **Not yet mapped**
/// (TODO 5.2b-next): enum-coded identity (nationality / handedness / birthplace) and player style, and
/// the `Match(existing)` update path. Returns the op + a skip report.
pub fn lower_create(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    person: &PersonSeason,
    target_team: u64,
) -> Option<(ImportOp, Vec<SkippedField>)> {
    let bio = db.table_canonical(ids::BIO)?;
    let mut writes: Vec<ImportWrite> = Vec::new();
    let mut skipped: Vec<SkippedField> = Vec::new();

    // Identity strings (inline, no offset).
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

    // Ratings on the kind-correct companion (Spike S1 rating edit_offset).
    let companion_id = if person.position == 4 { ids::GOALIE_AI } else { ids::SKATER_AI };
    if let Some(comp) = db.table_canonical(companion_id) {
        for (fid, human) in &person.ratings {
            push_semantic_int(db, meta, enums, comp, ImportTarget::Companion, &mut writes, &mut skipped, fid, *human);
        }
    }

    Some((ImportOp::Create { team: target_team, position: person.position, writes }, skipped))
}
