//! Phase 5.0 **Spike S2** — EA match-key DOB coverage + name-collision ambiguity.
//!
//! The Roster Builder matches an FHM person to an existing EA bio by **normalized name + DOB**
//! (`tdb_fhm::slug`). This spike measures, over the committed anchors, how usable that key is:
//!
//! * **DOB coverage** — what fraction of EA bios carry a *plausible* birthdate vs a zero/sentinel
//!   date (a raw-zero bio composes to a ~1900 date, not an empty string, so "missing" must be
//!   detected by plausibility, not `Option::None`).
//! * **Name-only ambiguity** — the collision rate of `normalize_name(last)|normalize_name(first)`
//!   alone, which is what the matcher must fall back to when a DOB is absent.
//! * **Name+DOB ambiguity** — the residual collision rate once DOB is folded in (the real key).
//!
//! It reuses the **exact app read path** (`EditSession::player_summaries` / `player_view`), so the
//! birthdate is composed with the correct per-title probe (year base + the NHL10 month/day
//! 1-index drift) rather than a re-derived offset.
//!
//! Run with output: `cargo test -p studio-domain --test ea_match_key -- --nocapture`.
//! The printed table is transcribed into `docs/audits/ea-match-key.md` with provenance.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use studio_domain::{EditSession, Platform, Project, ProjectRole};
use tdb_fhm::slug::{normalize_name, person_slug};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// A real NHL/AHL bio is born in this window; a raw-zero date lands at ~1900 and a raw-`0xFF`
/// sentinel lands well past 2100, so both fall outside it. Deliberately generous on the low end
/// to keep any legends-pack births in-scope while still excluding the base-year sentinel.
const DOB_MIN_YEAR: i32 = 1940;
const DOB_MAX_YEAR: i32 = 2010;

struct Anchor {
    title: &'static str,
    platform: Platform,
    db: &'static str,
    meta: &'static str,
}

fn matrix() -> Vec<Anchor> {
    let mut v = Vec::new();
    // NHL07 gen6 (bespoke NHL07NG.db) — schema differs; included to see whether it enumerates.
    v.push(Anchor {
        title: "nhl07/ps3",
        platform: Platform::Ps3,
        db: "anchors/ps3/nhl07/databases/NHL07NG.db",
        meta: "anchors/ps3/nhl07/databases/NHL07NG-meta.xml",
    });
    for t in [
        "nhl08",
        "nhl09",
        "nhl10",
        "nhl11",
        "nhl12",
        "nhl13",
        "nhl14",
        "nhl15",
        "nhllegacy",
    ] {
        v.push(Anchor {
            title: Box::leak(format!("{t}/ps3").into_boxed_str()),
            platform: Platform::Ps3,
            db: Box::leak(format!("anchors/ps3/{t}/databases/nhlng.db").into_boxed_str()),
            meta: Box::leak(format!("anchors/ps3/{t}/databases/nhlng-meta.xml").into_boxed_str()),
        });
    }
    for t in [
        "nhl15", "nhl16", "nhl17", "nhl18", "nhl19", "nhl20", "nhl21",
    ] {
        v.push(Anchor {
            title: Box::leak(format!("{t}/ps4").into_boxed_str()),
            platform: Platform::Ps4,
            db: Box::leak(format!("anchors/ps4/{t}/databases/nhl_game.db").into_boxed_str()),
            meta: Box::leak(
                format!("anchors/ps4/{t}/databases/nhl_game-meta.xml").into_boxed_str(),
            ),
        });
    }
    v
}

#[derive(Default)]
struct Stats {
    players: usize,
    dob_plausible: usize,
    dob_sentinel: usize, // present-but-out-of-window (zero/1900 or 0xFF/2100+)
    name_only_ambiguous: usize,
    name_dob_ambiguous: usize,
}

/// Parse the leading `YYYY` out of a composed `YYYY-MM-DD`.
fn year_of(bd: &str) -> Option<i32> {
    bd.split('-').next()?.trim().parse::<i32>().ok()
}

fn measure(a: &Anchor) -> Result<Stats, String> {
    let db_path = repo(a.db);
    if !db_path.exists() {
        return Err("db missing".into());
    }
    let project = Project::load(
        &db_path,
        a.platform,
        ProjectRole::Nhlng,
        repo(a.meta),
        repo("defs"),
    )
    .map_err(|e| format!("load: {e}"))?;
    let sess = EditSession::begin(project);
    let summaries = sess
        .player_summaries()
        .map_err(|e| format!("summaries: {e}"))?;

    let mut st = Stats {
        players: summaries.len(),
        ..Default::default()
    };
    // key -> count, for both the name-only and name+dob folds.
    let mut name_only: HashMap<String, u32> = HashMap::new();
    let mut name_dob: HashMap<String, u32> = HashMap::new();
    // Retain per-player keys so we can count how many players sit in a colliding bucket.
    let mut per_player: Vec<(String, String)> = Vec::with_capacity(summaries.len());

    for s in &summaries {
        let bd = sess
            .player_view(s.player_id)
            .ok()
            .flatten()
            .and_then(|v| v.header.birthdate);
        let plausible_dob = bd
            .as_deref()
            .and_then(year_of)
            .filter(|y| (DOB_MIN_YEAR..=DOB_MAX_YEAR).contains(y))
            .is_some();
        if let Some(bd) = &bd {
            match year_of(bd) {
                Some(y) if (DOB_MIN_YEAR..=DOB_MAX_YEAR).contains(&y) => st.dob_plausible += 1,
                Some(_) => st.dob_sentinel += 1,
                None => {}
            }
        }

        let name_key = format!(
            "{}|{}",
            normalize_name(&s.last_name),
            normalize_name(&s.first_name)
        );
        // Only fold in DOB when it is plausible; otherwise the key degrades to name-only (the
        // matcher's real fallback), which is exactly what we want to measure.
        let dob_str = if plausible_dob {
            bd.unwrap_or_default()
        } else {
            String::new()
        };
        let full_key = person_slug(&s.last_name, &s.first_name, &dob_str);

        *name_only.entry(name_key.clone()).or_insert(0) += 1;
        *name_dob.entry(full_key.clone()).or_insert(0) += 1;
        per_player.push((name_key, full_key));
    }

    for (nk, fk) in &per_player {
        if name_only.get(nk).copied().unwrap_or(0) >= 2 {
            st.name_only_ambiguous += 1;
        }
        if name_dob.get(fk).copied().unwrap_or(0) >= 2 {
            st.name_dob_ambiguous += 1;
        }
    }
    Ok(st)
}

// On-demand spike harness, **not** a gate: it opens every anchor and calls `player_view` per
// player (which rebuilds the entity model each call), so a full sweep runs for over an hour.
// Run explicitly: `cargo test -p studio-domain --test ea_match_key -- --ignored --nocapture`.
#[test]
#[ignore = "spike harness: full-matrix sweep runs >1h; run on demand with --ignored"]
fn ea_match_key_coverage() {
    let mut any = false;
    println!(
        "\n{:<14} {:>7} {:>10} {:>9} {:>13} {:>13}",
        "anchor", "players", "dob_ok%", "sentinel", "name-ambig%", "name+dob-ambig%"
    );
    println!("{}", "-".repeat(74));
    for a in matrix() {
        match measure(&a) {
            Ok(st) if st.players > 0 => {
                any = true;
                let pct = |n: usize| 100.0 * n as f64 / st.players as f64;
                println!(
                    "{:<14} {:>7} {:>9.1} {:>9} {:>12.1} {:>15.1}",
                    a.title,
                    st.players,
                    pct(st.dob_plausible),
                    st.dob_sentinel,
                    pct(st.name_only_ambiguous),
                    pct(st.name_dob_ambiguous),
                );
            }
            Ok(_) => println!("{:<14} {:>7}  (no players enumerated)", a.title, 0),
            Err(e) => println!("{:<14}   SKIP: {e}", a.title),
        }
    }
    println!();
    assert!(any, "expected at least one anchor to enumerate players");
}
