//! Calendar-year display and editing across the supported generation-seven anchors.
use std::path::Path;
use studio_api::Registry;
use studio_domain::{FieldValue, Platform};
#[test]
fn draft_years_across_titles_and_platforms() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for game in [
        "nhl10",
        "nhl11",
        "nhl12",
        "nhl13",
        "nhl14",
        "nhl15",
        "nhllegacy",
    ] {
        for (platform, label) in [(Platform::Xbox, "xbox360"), (Platform::Ps3, "ps3")] {
            let reg = Registry::default();
            let d = root.join(format!("anchors/{label}/{game}/databases"));
            let id = reg
                .open_workspace_from_paths(
                    Some(game),
                    platform,
                    d.join("nhlng.db").to_str().unwrap(),
                    None,
                    d.join("nhlng-meta.xml").to_str().unwrap(),
                    root.join("defs").to_str().unwrap(),
                )
                .unwrap()
                .project_id;
            let players = reg.list_players(id).unwrap();
            for (name, year) in [("Crosby", 2005), ("Ovechkin", 2004), ("Thornton", 1997)] {
                let p = players
                    .iter()
                    .find(|p| p.last_name == name && (name != "Thornton" || p.first_name == "Joe"))
                    .unwrap();
                let view = reg.player_view(id, p.player_id).unwrap().unwrap();
                let c = view
                    .profile
                    .sections
                    .iter()
                    .flat_map(|s| &s.cells)
                    .find(|c| c.label == "Draft Year")
                    .unwrap();
                let f = c.field.as_ref().unwrap();
                assert!(c.unverified.is_none(), "{game}/{label}: {:?}", c.unverified);
                assert_eq!(f.display, year.to_string(), "{game}/{label} {name}");
                assert_eq!(f.edit_offset, Some(1900));
                assert_eq!(view.data.draft_year, Some(year as u32));
                assert_eq!(f.decoded, Some(year - 1900));
                reg.set_field(
                    id,
                    &f.studio_table,
                    c.record,
                    &f.studio_field,
                    FieldValue::Int(2006 - 1900),
                )
                .unwrap();
                let edited = reg.player_view(id, p.player_id).unwrap().unwrap();
                let after = edited
                    .all_fields
                    .iter()
                    .find(|v| v.studio_field == f.studio_field)
                    .unwrap();
                assert_eq!(after.display, "2006");
                assert_eq!(after.decoded, Some(106));
                // Restore the original value in memory; committed anchor files are never written.
                reg.set_field(
                    id,
                    &f.studio_table,
                    c.record,
                    &f.studio_field,
                    FieldValue::Int(year - 1900),
                )
                .unwrap();
                let restored = reg.player_view(id, p.player_id).unwrap().unwrap();
                assert_eq!(
                    restored
                        .all_fields
                        .iter()
                        .find(|v| v.studio_field == f.studio_field)
                        .unwrap()
                        .display,
                    year.to_string()
                );
                println!("{game}/{label} {name}: {year}, edit and restore passed");
            }
        }
    }
}
