//! League 1.2 (receipt 44; research R133, 2026-09-26 handoff 26 §4): conference / division labels on the NHL 08–11
//! local-bit titles. NHL 10 / 11 conferences carry the executables' names by position as **inferred** candidates;
//! divisions are never named — they are labelled by the clubs this title files there. NHL 08 / 09 (no name list
//! read) keep conference numbers. NHL 07 (its own vocabulary) and NHL 12 (global group table) are unchanged.
use std::path::{Path, PathBuf};

use studio_domain::{EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn open(game: &str, platform: Platform) -> EditSession {
    let dir = if platform == Platform::Ps3 { "ps3" } else { "xbox360" };
    let (db, meta) = if game == "nhl07" { ("NHL07NG.db", "NHL07NG-meta.xml") } else { ("nhlng.db", "nhlng-meta.xml") };
    let base = repo(&format!("anchors/{dir}/{game}/databases"));
    let mut s = EditSession::begin(Project::load(base.join(db), platform, ProjectRole::Nhlng, base.join(meta), repo("defs")).unwrap());
    s.set_game_id(Some(game.to_string()));
    s
}

/// (conference label, division label) of the team with this abbreviation.
fn labels(s: &EditSession, abbrev: &str) -> (Option<String>, Option<String>, u64) {
    let t = s.team_summaries().unwrap().into_iter().find(|t| t.abbrev == abbrev).unwrap_or_else(|| panic!("{abbrev}"));
    let v = s.team_view(t.team_id).unwrap().unwrap();
    (v.conference_label, v.division_label, t.team_id)
}

#[test]
fn conferences_by_position_on_10_11_and_divisions_by_members_on_08_11() {
    for (game, platform) in [("nhl08", Platform::Xbox), ("nhl09", Platform::Xbox), ("nhl10", Platform::Xbox), ("nhl10", Platform::Ps3), ("nhl11", Platform::Xbox), ("nhl11", Platform::Ps3)] {
        let s = open(game, platform);
        let (conf, div, chi) = labels(&s, "CHI");
        assert_eq!(div.as_deref(), Some("Division 3: CBJ · CHI · DET · NSH · STL"), "{game} {platform:?}");
        let (tconf, tdiv, _) = labels(&s, "TOR");
        assert_eq!(tdiv.as_deref(), Some("Division 0: BOS · BUF · MTL · OTT · TOR"), "{game}: the All-Star rows (conference 0) stay apart");
        if matches!(game, "nhl10" | "nhl11") {
            assert_eq!(conf.as_deref(), Some("Western Conference (inferred)"), "{game}");
            assert_eq!(tconf.as_deref(), Some("Eastern Conference (inferred)"), "{game}");
        } else {
            assert_eq!((conf.as_deref(), tconf.as_deref()), (None, None), "{game}: no name list read on this title — numbers stay");
        }
        // The membership editor shows the same division label (with the untraced note).
        let o = studio_domain::membership::options(&s, chi).unwrap();
        let d = o.current.division.as_ref().unwrap();
        assert!(d.label.starts_with("Division 3: CBJ · CHI") && d.label.ends_with("(name untraced)"), "{game}: {}", d.label);
        eprintln!("{game} {platform:?}: CHI = {conf:?} / {div:?}; TOR = {tconf:?} / {tdiv:?}; editor: {}", d.label);
    }
    // NHL 11: the QMJHL block has no conference names — nothing is assigned there.
    let s = open("nhl11", Platform::Xbox);
    let q = s.team_summaries().unwrap().into_iter().find(|t| t.league == Some(9)).unwrap();
    let v = s.team_view(q.team_id).unwrap().unwrap();
    assert!(v.conference_label.is_none(), "QMJHL conference stays a number: {:?}", v.conference_label);
    assert!(v.division_label.as_deref().is_some_and(|d| d.starts_with("Division ")), "{:?}", v.division_label);
    eprintln!("nhl11 QMJHL {}: {:?} / {:?}", q.abbrev, v.conference_label, v.division_label);
    // NHL 07 keeps its vocabulary labels; NHL 12 its group table.
    // NHL 07 keeps its vocabulary labels, read from the stock row (receipt 44 defect: the thin selector's
    // all-zero `division` column labelled every club "Northeast").
    let s = open("nhl07", Platform::Xbox);
    let (_, div07, _) = labels(&s, "CHI");
    let (_, tor07, _) = labels(&s, "TOR");
    let (_, ana07, _) = labels(&s, "ANA");
    assert_eq!(tor07.as_deref(), Some("Northeast"), "NHL 07 division 0");
    assert!(div07.is_some() && div07 != tor07 && ana07 != tor07 && ana07 != div07, "NHL 07 CHI / ANA / TOR in three divisions: {div07:?} {ana07:?} {tor07:?}");
    let s = open("nhl12", Platform::Xbox);
    let (c12, d12, _) = labels(&s, "CHI");
    assert!(d12.as_deref().is_some_and(|d| !d.starts_with("Division ")), "NHL 12 keeps its group table: {c12:?} / {d12:?}");
    eprintln!("nhl07 CHI division {div07:?}; nhl12 CHI {c12:?} / {d12:?}");
}

#[test]
#[ignore = "diagnostic listing (receipt 44): NHL 07 division labels per club"]
fn nhl07_division_labels_listing() {
    let s = open("nhl07", Platform::Xbox);
    for t in s.team_summaries().unwrap().into_iter().filter(|t| t.league == Some(0)) {
        let v = s.team_view(t.team_id).unwrap().unwrap();
        eprintln!("{} {} rec {}: {:?} / {:?}", t.abbrev, t.team_id, t.record, v.conference_label, v.division_label);
    }
}
