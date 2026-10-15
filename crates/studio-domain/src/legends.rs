//! Legends roster assignment — turning a field of Legends-rated players into the builder's team
//! rosters.
//!
//! The rules (operator spec):
//!  1. A player needs **200+ NHL regular-season career games** to be considered for a franchise team.
//!  2. They are assigned by default to the franchise they played the most NHL games for.
//!  3. Each franchise fields the top **15 forwards, 8 defensemen, 3 goalies**.
//!  4. If their most-tenured franchise is already full at their position, they fall to their
//!     second-most-tenured franchise — but only with **150+ games** there. There is no third fall.
//!  5. Two residual teams then take everyone who didn't make a franchise team, *regardless of NHL
//!     games*, so pre-NHL and non-NHL greats are still reachable:
//!     - **European Elite** — European players.
//!     - **Windsor Legends** — North American players.
//!
//! Placement is a single global pass in descending Legends Overall, so the best player always gets
//! first claim on their own franchise and crowding only ever pushes the weaker player down.

use std::collections::HashMap;

use tdb_fhm_store::legends::{FranchiseTenure, LegendsCandidate};

/// Career NHL regular-season games needed to be considered for a franchise team.
pub const MIN_CAREER_NHL_GP: i64 = 200;
/// Games needed with the *second* franchise to fall back onto it.
pub const MIN_SECOND_TEAM_GP: i64 = 150;

/// Per-team positional capacity.
pub const MAX_FORWARDS: usize = 15;
pub const MAX_DEFENSEMEN: usize = 8;
pub const MAX_GOALIES: usize = 3;

/// Team key for the European residual team.
pub const EUROPEAN_ELITE_KEY: &str = "european-elite";
/// Team key for the North American residual team.
pub const WINDSOR_LEGENDS_KEY: &str = "windsor-legends";

pub const EUROPEAN_ELITE_NAME: &str = "European Elite";
pub const WINDSOR_LEGENDS_NAME: &str = "Windsor Legends";

/// The three roster slots a team fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Forward,
    Defense,
    Goalie,
}

impl Slot {
    /// Bucket a candidate. `is_goalie` wins over the position label (the corpus sets both, and the
    /// flag is the authoritative one); otherwise `D` is defense and every other skater label is a
    /// forward. Unknown labels return `None` and the player is simply never placed.
    pub fn of(c: &LegendsCandidate) -> Option<Slot> {
        if c.is_goalie {
            return Some(Slot::Goalie);
        }
        match c.position.trim().to_uppercase().as_str() {
            "G" => Some(Slot::Goalie),
            "D" | "LD" | "RD" => Some(Slot::Defense),
            "C" | "LW" | "RW" | "F" => Some(Slot::Forward),
            _ => None,
        }
    }

    pub fn capacity(self) -> usize {
        match self {
            Slot::Forward => MAX_FORWARDS,
            Slot::Defense => MAX_DEFENSEMEN,
            Slot::Goalie => MAX_GOALIES,
        }
    }
}

/// Which residual team a player falls to when they miss every franchise team.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    NorthAmerica,
    Europe,
    /// Neither list claims this nationality — the player stays unplaced rather than being
    /// silently filed onto a team they don't belong to.
    Other,
}

/// Nationalities that field the Windsor Legends.
const NORTH_AMERICAN: &[&str] = &[
    "CANADA",
    "CAN",
    "UNITED STATES",
    "UNITED STATES OF AMERICA",
    "USA",
    "US",
    "AMERICA",
    "MEXICO",
    "MEX",
];

/// Nationalities that field the European Elite. Both the full name and FHM's 3-letter code are
/// listed because the corpus's `person.nationality` vocabulary varies by export vintage.
const EUROPEAN: &[&str] = &[
    "RUSSIA",
    "RUS",
    "SOVIET UNION",
    "USSR",
    "URS",
    "SWEDEN",
    "SWE",
    "FINLAND",
    "FIN",
    "NORWAY",
    "NOR",
    "DENMARK",
    "DEN",
    "ICELAND",
    "ISL",
    "CZECHIA",
    "CZECH REPUBLIC",
    "CZE",
    "CZECHOSLOVAKIA",
    "TCH",
    "SLOVAKIA",
    "SVK",
    "SWITZERLAND",
    "SUI",
    "SWI",
    "GERMANY",
    "GER",
    "DEU",
    "EAST GERMANY",
    "WEST GERMANY",
    "AUSTRIA",
    "AUT",
    "LATVIA",
    "LAT",
    "LTU",
    "LITHUANIA",
    "ESTONIA",
    "EST",
    "BELARUS",
    "BLR",
    "UKRAINE",
    "UKR",
    "POLAND",
    "POL",
    "SLOVENIA",
    "SLO",
    "SVN",
    "CROATIA",
    "CRO",
    "SERBIA",
    "SRB",
    "YUGOSLAVIA",
    "YUG",
    "HUNGARY",
    "HUN",
    "ROMANIA",
    "ROU",
    "RUM",
    "BULGARIA",
    "BUL",
    "ITALY",
    "ITA",
    "FRANCE",
    "FRA",
    "NETHERLANDS",
    "NED",
    "NLD",
    "BELGIUM",
    "BEL",
    "UNITED KINGDOM",
    "GREAT BRITAIN",
    "GBR",
    "ENGLAND",
    "SCOTLAND",
    "IRELAND",
    "IRL",
    "SPAIN",
    "ESP",
    "KAZAKHSTAN",
    "KAZ",
];

impl Region {
    /// Classify a corpus nationality string.
    ///
    /// FLAGGED: the corpus's nationality vocabulary hasn't been enumerated against a real data pack
    /// yet, so both full names and 3-letter codes are matched and anything unrecognised lands in
    /// [`Region::Other`] rather than being guessed onto a team. [`LegendsAssignment::unclassified`]
    /// reports what fell through so the lists can be corrected from real data.
    pub fn of(nationality: Option<&str>) -> Region {
        let Some(n) = nationality else {
            return Region::Other;
        };
        let key = n.trim().to_uppercase();
        if key.is_empty() {
            return Region::Other;
        }
        if NORTH_AMERICAN.contains(&key.as_str()) {
            return Region::NorthAmerica;
        }
        if EUROPEAN.contains(&key.as_str()) {
            return Region::Europe;
        }
        Region::Other
    }
}

/// One player's Legends rating, as the assignment engine needs it.
#[derive(Debug, Clone, Copy)]
pub struct RatedPlayer {
    pub person_id: i64,
    pub overall: i64,
}

/// A player placed onto a team.
#[derive(Debug, Clone)]
pub struct LegendsMember {
    pub person_id: i64,
    pub overall: i64,
    pub slot: Slot,
    /// NHL regular-season games with the franchise they were placed on (0 on a residual team).
    pub team_gp: i64,
    /// True when they were crowded off their most-tenured franchise onto their second.
    pub second_choice: bool,
}

/// One built team.
#[derive(Debug, Clone)]
pub struct LegendsTeam {
    /// `franchise:<id>` for a real franchise, or one of the two residual keys.
    pub key: String,
    pub franchise_id: Option<i64>,
    pub forwards: Vec<LegendsMember>,
    pub defense: Vec<LegendsMember>,
    pub goalies: Vec<LegendsMember>,
}

impl LegendsTeam {
    fn new(key: String, franchise_id: Option<i64>) -> Self {
        Self {
            key,
            franchise_id,
            forwards: Vec::new(),
            defense: Vec::new(),
            goalies: Vec::new(),
        }
    }

    fn bucket(&mut self, slot: Slot) -> &mut Vec<LegendsMember> {
        match slot {
            Slot::Forward => &mut self.forwards,
            Slot::Defense => &mut self.defense,
            Slot::Goalie => &mut self.goalies,
        }
    }

    fn has_room(&mut self, slot: Slot) -> bool {
        self.bucket(slot).len() < slot.capacity()
    }

    /// Every member, forwards then defense then goalies.
    pub fn members(&self) -> impl Iterator<Item = &LegendsMember> {
        self.forwards
            .iter()
            .chain(self.defense.iter())
            .chain(self.goalies.iter())
    }
}

/// The full build: every team, plus what the pass couldn't place.
#[derive(Debug, Clone, Default)]
pub struct LegendsAssignment {
    /// Keyed by [`LegendsTeam::key`].
    pub teams: HashMap<String, LegendsTeam>,
    /// Players who qualified but found no team (both franchises full, and their nationality claimed
    /// by neither residual team).
    pub unplaced: Vec<i64>,
    /// Unplaced players whose nationality matched neither region list — the signal that
    /// [`Region::of`]'s vocabulary needs widening against the real corpus.
    pub unclassified: Vec<i64>,
}

/// Build every Legends roster.
///
/// `candidates` is the corpus's whole rated population, `ratings` their Legends Overalls, and
/// `tenure` the per-franchise NHL games (any order — it's re-sorted per player here).
pub fn assign_rosters(
    candidates: &[LegendsCandidate],
    ratings: &HashMap<i64, i64>,
    tenure: &[FranchiseTenure],
) -> LegendsAssignment {
    // Per-player franchise tenure, most games first. Ties break on franchise id so a player with two
    // identical stints lands somewhere deterministic instead of shuffling between builds.
    let mut by_person: HashMap<i64, Vec<(i64, i64)>> = HashMap::new();
    for t in tenure {
        by_person
            .entry(t.person_id)
            .or_default()
            .push((t.franchise_id, t.gp));
    }
    for v in by_person.values_mut() {
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    }

    // The global draft order: best Legends Overall first. Ties break on career games (the longer
    // career wins the slot), then person id for determinism.
    let mut ranked: Vec<(&LegendsCandidate, i64, Slot)> = candidates
        .iter()
        .filter_map(|c| {
            let ovr = *ratings.get(&c.person_id)?;
            let slot = Slot::of(c)?;
            Some((c, ovr, slot))
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| b.0.nhl_gp.cmp(&a.0.nhl_gp))
            .then_with(|| a.0.person_id.cmp(&b.0.person_id))
    });

    let mut out = LegendsAssignment::default();
    out.teams.insert(
        EUROPEAN_ELITE_KEY.to_string(),
        LegendsTeam::new(EUROPEAN_ELITE_KEY.to_string(), None),
    );
    out.teams.insert(
        WINDSOR_LEGENDS_KEY.to_string(),
        LegendsTeam::new(WINDSOR_LEGENDS_KEY.to_string(), None),
    );

    // ---- Pass 1: franchise teams (200+ NHL GP only) -----------------------
    let mut leftovers: Vec<(&LegendsCandidate, i64, Slot)> = Vec::new();
    for &(c, ovr, slot) in &ranked {
        if c.nhl_gp < MIN_CAREER_NHL_GP {
            leftovers.push((c, ovr, slot));
            continue;
        }
        let stints = by_person
            .get(&c.person_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        // First choice: the most-tenured franchise. Second choice: the next one, and only when the
        // player logged real time there (150+ games) — a 12-game cameo is not a Legends home.
        let mut placed = false;
        for (idx, &(franchise_id, gp)) in stints.iter().take(2).enumerate() {
            if idx == 1 && gp < MIN_SECOND_TEAM_GP {
                break;
            }
            let key = franchise_key(franchise_id);
            let team = out
                .teams
                .entry(key.clone())
                .or_insert_with(|| LegendsTeam::new(key.clone(), Some(franchise_id)));
            if team.has_room(slot) {
                team.bucket(slot).push(LegendsMember {
                    person_id: c.person_id,
                    overall: ovr,
                    slot,
                    team_gp: gp,
                    second_choice: idx == 1,
                });
                placed = true;
                break;
            }
        }
        if !placed {
            leftovers.push((c, ovr, slot));
        }
    }

    // ---- Pass 2: the residual teams (no games threshold) ------------------
    for (c, ovr, slot) in leftovers {
        let key = match Region::of(c.nationality.as_deref()) {
            Region::Europe => EUROPEAN_ELITE_KEY,
            Region::NorthAmerica => WINDSOR_LEGENDS_KEY,
            Region::Other => {
                out.unplaced.push(c.person_id);
                out.unclassified.push(c.person_id);
                continue;
            }
        };
        let team = out
            .teams
            .get_mut(key)
            .expect("residual teams are pre-seeded");
        if team.has_room(slot) {
            team.bucket(slot).push(LegendsMember {
                person_id: c.person_id,
                overall: ovr,
                slot,
                team_gp: 0,
                second_choice: false,
            });
        } else {
            out.unplaced.push(c.person_id);
        }
    }

    out
}

/// The team key for a franchise lineage.
pub fn franchise_key(franchise_id: i64) -> String {
    format!("franchise:{franchise_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(person_id: i64, position: &str, nhl_gp: i64, nationality: &str) -> LegendsCandidate {
        LegendsCandidate {
            person_id,
            first_name: format!("P{person_id}"),
            last_name: "Test".into(),
            position: position.into(),
            is_goalie: position == "G",
            nationality: Some(nationality.into()),
            dob: "1950-01-01".into(),
            nhl_gp,
        }
    }

    fn tenure(person_id: i64, franchise_id: i64, gp: i64) -> FranchiseTenure {
        FranchiseTenure {
            person_id,
            franchise_id,
            gp,
        }
    }

    #[test]
    fn region_matches_names_and_codes() {
        assert_eq!(Region::of(Some("Canada")), Region::NorthAmerica);
        assert_eq!(Region::of(Some("CAN")), Region::NorthAmerica);
        assert_eq!(Region::of(Some("sweden")), Region::Europe);
        assert_eq!(Region::of(Some("USSR")), Region::Europe);
        assert_eq!(Region::of(Some("Japan")), Region::Other);
        assert_eq!(Region::of(None), Region::Other);
    }

    #[test]
    fn under_the_games_threshold_falls_to_a_residual_team() {
        let cands = vec![cand(1, "C", 199, "Canada"), cand(2, "C", 200, "Canada")];
        let ratings = HashMap::from([(1, 95), (2, 60)]);
        let t = vec![tenure(1, 10, 199), tenure(2, 10, 200)];
        let a = assign_rosters(&cands, &ratings, &t);

        // The 95 with 199 games misses the franchise team despite outranking everyone.
        assert!(a.teams[&franchise_key(10)]
            .forwards
            .iter()
            .all(|m| m.person_id == 2));
        assert!(a.teams[WINDSOR_LEGENDS_KEY]
            .forwards
            .iter()
            .any(|m| m.person_id == 1));
    }

    #[test]
    fn overflow_falls_to_the_second_franchise_only_above_150_games() {
        // 16 forwards all most-tenured at franchise 10; the 16th (lowest rated) overflows.
        let mut cands: Vec<LegendsCandidate> = Vec::new();
        let mut ratings = HashMap::new();
        let mut t = Vec::new();
        for i in 1..=16 {
            cands.push(cand(i, "C", 500, "Canada"));
            ratings.insert(i, 100 - i); // 1 is best, 16 is worst
            t.push(tenure(i, 10, 400));
        }
        // The worst forward also has a real second stint; give it enough games to qualify.
        t.push(tenure(16, 20, 150));
        // Another has a second stint that is too short to fall back onto.
        cands.push(cand(17, "C", 500, "Canada"));
        ratings.insert(17, 50);
        t.push(tenure(17, 10, 400));
        t.push(tenure(17, 30, 149));

        let a = assign_rosters(&cands, &ratings, &t);
        assert_eq!(a.teams[&franchise_key(10)].forwards.len(), MAX_FORWARDS);
        // #16 lands on franchise 20 as a second choice.
        let second = &a.teams[&franchise_key(20)].forwards;
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].person_id, 16);
        assert!(second[0].second_choice);
        // #17's 149-game stint is below the bar, so they drop to the residual team instead.
        assert!(!a.teams.contains_key(&franchise_key(30)));
        assert!(a.teams[WINDSOR_LEGENDS_KEY]
            .forwards
            .iter()
            .any(|m| m.person_id == 17));
    }

    #[test]
    fn residual_teams_take_non_nhl_players_by_region() {
        let cands = vec![
            cand(1, "G", 0, "Russia"), // never played in the NHL
            cand(2, "D", 0, "Canada"),
            cand(3, "C", 0, "Japan"), // claimed by neither list
        ];
        let ratings = HashMap::from([(1, 90), (2, 88), (3, 99)]);
        let a = assign_rosters(&cands, &ratings, &[]);

        assert_eq!(a.teams[EUROPEAN_ELITE_KEY].goalies[0].person_id, 1);
        assert_eq!(a.teams[WINDSOR_LEGENDS_KEY].defense[0].person_id, 2);
        assert_eq!(a.unclassified, vec![3]);
    }

    #[test]
    fn capacity_is_enforced_per_position() {
        let mut cands = Vec::new();
        let mut ratings = HashMap::new();
        let mut t = Vec::new();
        for i in 1..=20 {
            cands.push(cand(i, "G", 500, "Canada"));
            ratings.insert(i, 100 - i);
            t.push(tenure(i, 7, 500));
        }
        let a = assign_rosters(&cands, &ratings, &t);
        assert_eq!(a.teams[&franchise_key(7)].goalies.len(), MAX_GOALIES);
        // The top 3 by Overall got the slots.
        let ids: Vec<i64> = a.teams[&franchise_key(7)]
            .goalies
            .iter()
            .map(|m| m.person_id)
            .collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }
}
