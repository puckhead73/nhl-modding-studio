//! Receipt 67 (research 2026-09-28 handoff 12 §2): a dictionary that cannot be attached at open is recorded and disclosed, never
//! dropped silently; a good attach clears it. Anchors only, read only.
use std::path::{Path, PathBuf};

use studio_domain::{EditSession, Platform, Project, ProjectRole, Workspace};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

#[test]
fn a_failed_attach_at_open_is_recorded_and_a_good_one_clears_it() {
    let d = repo("anchors/xbox360/nhllegacy/databases");
    let session = EditSession::begin(Project::load(d.join("nhlng.db"), Platform::Xbox, ProjectRole::Nhlng, d.join("nhlng-meta.xml"), repo("defs")).unwrap());
    let mut ws = Workspace::single(session, d.join("nhlng.db").to_string_lossy().into_owned());
    assert_eq!(ws.dictionary_problem(), None);
    let missing = d.join("no_such_dictionary.db");
    ws.attach_dictionary_at_open(&missing.to_string_lossy());
    let p = ws.dictionary_problem().expect("recorded");
    assert!(p.contains("could not be attached") && p.contains("show as keys"), "{p}");
    assert!(ws.dictionary().is_none());
    ws.attach_dictionary_at_open(&d.join("nhl_eng_us.db").to_string_lossy());
    assert_eq!(ws.dictionary_problem(), None);
    assert!(ws.dictionary().is_some());
}
