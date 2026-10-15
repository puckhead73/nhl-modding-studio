//! Save Editor identification over the committed anchors: every available target's own main
//! database and roster save must identify as that target (among the best-evidence candidates),
//! and every anchor save fixture is classified without a panic. Run with `--nocapture` for the
//! full matrix.

use std::path::{Path, PathBuf};

use studio_domain::baselines::{BaselineStore, ROLE_MAIN_DB, ROLE_ROSTER_SAVE};
use studio_domain::save_identify::{OpenPolicy, SchemaIndex};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn store() -> BaselineStore {
    let r = repo();
    BaselineStore::open(&r.join("baselines/INVENTORY.json"), &r.join("anchors")).unwrap()
}

fn leaders(id: &studio_domain::save_identify::Identification) -> Vec<String> {
    let top = id.candidates.first().map(|c| c.score);
    id.candidates
        .iter()
        .filter(|c| Some(c.score) == top)
        .map(|c| format!("{}/{}", c.game_id, c.platform.as_str()))
        .collect()
}

#[test]
fn every_anchor_main_database_and_roster_save_identifies_as_its_own_target() {
    let store = store();
    let index = SchemaIndex::build(&store).unwrap();
    let mut failures = Vec::new();
    for t in store.inventory.targets.iter().filter(|t| t.is_available()) {
        for role in [ROLE_MAIN_DB, ROLE_ROSTER_SAVE] {
            let Some(c) = t.component(role) else { continue };
            let path = store.component_path(c);
            let want = format!("{}/{}", t.game_id, t.platform);
            match index.identify(&path) {
                Ok(id) => {
                    let lead = leaders(&id);
                    println!(
                        "{want:<20} {role:<12} kind={:<9} exact={} change_list={} game_confident={} platform_confident={} leaders={lead:?}",
                        id.probe.kind,
                        id.candidates.first().is_some_and(|c| c.exact_schema),
                        id.change_list,
                        id.game_confident,
                        id.platform_confident,
                    );
                    if !lead.contains(&want) {
                        failures.push(format!("{want} {role}: leaders {lead:?}"));
                    }
                    if !id.game_confident {
                        failures.push(format!("{want} {role}: game not confident"));
                    }
                    if id.open_policy() != OpenPolicy::Allowed {
                        failures.push(format!("{want} {role}: policy {:?}", id.open_policy()));
                    }
                }
                Err(e) => failures.push(format!("{want} {role}: {e}")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "misidentified anchors:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_anchor_save_fixture_is_classified() {
    let index = SchemaIndex::build(&store()).unwrap();
    let mut files = Vec::new();
    let mut stack = vec![repo().join("anchors")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.components().any(|c| c.as_os_str() == "savefiles") {
                let name = p
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                let aux = name.ends_with(".png")
                    || name.ends_with(".sfo")
                    || name == "index"
                    || name == "name";
                if !aux {
                    files.push(p);
                }
            }
        }
    }
    files.sort();
    assert!(!files.is_empty());
    let mut decisions = std::collections::BTreeMap::new();
    for p in files {
        let rel = p
            .strip_prefix(repo().join("anchors"))
            .unwrap()
            .display()
            .to_string()
            .replace('\\', "/");
        match index.identify(&p) {
            Ok(id) => {
                let policy = id.open_policy();
                println!(
                    "{rel:<75} {:<9} {:<4} exp={} leaders={:?} -> {}",
                    id.probe.kind,
                    id.probe.container_version.as_deref().unwrap_or("-"),
                    id.probe.write_experimental,
                    leaders(&id),
                    match &policy {
                        OpenPolicy::Allowed => "allowed",
                        OpenPolicy::Caution(_) => "caution",
                        OpenPolicy::Refused(_) => "refused",
                    }
                );
                // No settings/profile/Ultimate Team save ever opens.
                if ["settings", "hockeycard"].contains(&id.probe.kind.as_str()) {
                    assert!(
                        matches!(policy, OpenPolicy::Refused(_)),
                        "{rel} must be refused"
                    );
                }
                decisions.insert(rel, (id.probe.kind.clone(), policy));
            }
            Err(e) => println!("{rel:<75} not opened: {e}"),
        }
    }
    let decision = |rel: &str| {
        decisions
            .get(rel)
            .unwrap_or_else(|| panic!("{rel} was not identified"))
            .clone()
    };
    // NHL 07 / 08 roster saves are change lists against the game database: recognised and
    // opened applied to it.
    for rel in [
        "xbox360/nhl07/savefiles/ROSTER1/ROSTER1",
        "xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658",
        "ps3/nhl08/savefiles/BLUS30062-002-000 - ROSTER/SYS-DATA",
    ] {
        assert_eq!(decision(rel), ("roster".into(), OpenPolicy::Allowed), "{rel}");
    }
    // PS3 settings saves that carry a database are classified from their folder and refused.
    assert_eq!(
        decision("ps3/nhl13/savefiles/BLUS310030000 - SETTINGS/SYS-DATA").0,
        "settings"
    );
    // PS4 mode saves decode through the roster record but are classified from their NAME file.
    // The franchise / season / Be a Pro saves in the anchors hold their mode's tables, not the
    // rosters; they open over the game's default database (teams and players read-only).
    let franchise = decision("ps4/nhl15/savefiles/c6b05b60 - BEAGM/DATA");
    assert_eq!(franchise.0, "beagm");
    assert!(matches!(franchise.1, OpenPolicy::Caution(ref r) if r.contains("default database")));
    let threes = decision("ps4/nhl18/savefiles/ThreeOnThree0/DATA");
    assert_eq!(threes.0, "threes");
    assert!(matches!(threes.1, OpenPolicy::Caution(_)));
    for rel in [
        "xbox360/nhl10/savefiles/DYNASTY1 20260627045642/DYNASTY1 20260627045642",
        "ps3/nhl11/savefiles/BLUS305681100 - SEASON/SYS-DATA",
    ] {
        assert!(matches!(decision(rel).1, OpenPolicy::Caution(_)), "{rel}");
    }
    assert_eq!(
        decision("ps4/nhl17/savefiles/RosterCustom0/DATA"),
        ("roster".into(), OpenPolicy::Allowed)
    );
    // NHL 20 PS4 has no bundled definitions: refused rather than borrowed from a neighbour.
    assert!(
        matches!(decision("ps4/nhl20/savefiles/ThreeOnThree0/DATA").1, OpenPolicy::Refused(ref r) if r.contains("bundled game definitions"))
    );
    // An Xbox 360 roster save opens as its game.
    assert_eq!(
        decision("xbox360/nhl14/savefiles/ROSTER 20260627054746/ROSTER 20260627054746").1,
        OpenPolicy::Allowed
    );
}
