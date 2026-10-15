//! Phase 5.1 gate — `EaSlugIndex` / `match_player` over a real anchor (nhl14/ps3).
//!
//! A known EA player matches **themselves** on name+DOB (high confidence); the same lookup with no
//! DOB degrades to name-only review candidates; a made-up person returns nothing (→ Create). Uses the
//! anchor's own bios as the "incoming" identities, so no corpus is needed (the corpus cross-match is
//! Phase 5.2's bridge).

use std::path::{Path, PathBuf};

use studio_domain::{EditSession, MatchConfidence, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn open() -> EditSession {
    EditSession::begin(
        Project::load(
            repo("anchors/ps3/nhl14/databases/nhlng.db"),
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo("anchors/ps3/nhl14/databases/nhlng-meta.xml"),
            repo("defs"),
        )
        .unwrap(),
    )
}

#[test]
fn known_player_matches_self_on_name_and_dob() {
    let sess = open();
    let idx = sess.ea_slug_index().unwrap();
    assert!(
        (3000..7000).contains(&idx.len()),
        "implausible index size {}",
        idx.len()
    );

    // First anchor player that composes a usable DOB — our "incoming" identity.
    let summaries = sess.player_summaries().unwrap();
    let (pid, last, first, dob) = summaries
        .iter()
        .find_map(|s| {
            let bd = sess
                .player_view(s.player_id)
                .ok()
                .flatten()?
                .header
                .birthdate?;
            Some((s.player_id, s.last_name.clone(), s.first_name.clone(), bd))
        })
        .expect("a player with a composed DOB");

    let cands = idx.match_player(&last, &first, Some(&dob));
    let self_hit = cands
        .iter()
        .find(|c| c.ea_player_id == pid)
        .expect("player matches self");
    assert_eq!(
        self_hit.confidence,
        MatchConfidence::NameAndDob,
        "self-match is high confidence"
    );
    assert!(
        !self_hit.full_name.is_empty(),
        "candidate resolves a human-readable name"
    );
    assert_eq!(
        self_hit.dob.as_deref(),
        Some(dob.as_str()),
        "candidate carries the composed DOB"
    );
    // S2: name+DOB is not guaranteed unique — assert membership, not uniqueness.
    if cands.len() > 1 {
        assert!(
            cands.iter().all(|c| c.ambiguous),
            "a multi-candidate set flags every entry ambiguous"
        );
    }
}

#[test]
fn dobless_lookup_degrades_to_name_only() {
    let sess = open();
    let idx = sess.ea_slug_index().unwrap();
    let summaries = sess.player_summaries().unwrap();
    let s = summaries.first().expect("a player");

    let cands = idx.match_player(&s.last_name, &s.first_name, None);
    assert!(
        cands.iter().any(|c| c.ea_player_id == s.player_id),
        "the player is still found by name alone"
    );
    assert!(
        cands
            .iter()
            .all(|c| c.confidence == MatchConfidence::NameOnly),
        "no DOB → every candidate is name-only"
    );
}

#[test]
fn absent_person_returns_no_candidate() {
    let sess = open();
    let idx = sess.ea_slug_index().unwrap();
    assert!(
        idx.match_player("Qwertzuiop", "Zxcvbnm", Some("1990-06-15"))
            .is_empty(),
        "an unknown person yields no candidate → the cart should Create"
    );
    assert!(
        idx.match_player("Qwertzuiop", "Zxcvbnm", None).is_empty(),
        "unknown name, no DOB, still nothing"
    );
}
