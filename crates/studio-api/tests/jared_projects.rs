//! Local tester fixtures are extracted under local/bug-repro-jared; originals stay read-only.
use std::path::{Path, PathBuf};
use studio_api::{Registry, dto::PackageBuildRequestDto};
use studio_domain::Platform;
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
#[ignore = "requires Jared's private project fixtures"]
fn tester_projects_build_and_database_packages_activate_and_restore() {
    for (game, file, platform) in [("nhl07", "NHL07NG", Platform::Xbox), ("nhl08", "nhlng", Platform::Ps3)] {
        let root = repo(&format!("local/bug-repro-jared/{game}"));
        let defs = s(&repo("defs"));
        let reg = Registry::default();
        let id = reg.project_open(&s(&root), &defs).unwrap().summary.project_id;
        let temp = tempfile::tempdir().unwrap();
        let request = |include: Vec<String>, name: &str| PackageBuildRequestDto {
            name: format!("Jared {game}"), version: "1".into(), description: String::new(), include,
            exclude: if name == "database.nhlmod" { vec!["textures".into(), "dictionary".into()] } else { vec![] }, platforms: vec![platform.as_str().into()], out_path: Some(s(&temp.path().join(name))),
        };
        let all = reg.package_build(id, &request(vec!["cells".into(), "textures".into(), "dictionary".into()], "full.nhlmod"), None, &defs).unwrap();
        assert!(!reg.package_inspect(&all.path).unwrap().changes.is_empty());
        let built = reg.package_build(id, &request(vec!["cells".into()], "database.nhlmod"), None, &defs).unwrap();
        let target = temp.path().join("game");
        std::fs::create_dir_all(target.join("db")).unwrap();
        for ext in ["db", "xml"] {
            let name = if ext == "db" {format!("{file}.db")} else {format!("{file}-meta.xml")};
            std::fs::copy(root.join("baseline/db").join(&name), target.join("db").join(name)).unwrap();
        }
        let db = target.join(format!("db/{file}.db"));
        let before = std::fs::read(&db).unwrap();
        let plan = reg.package_preflight(&built.path, game, platform, &s(&target), None, None, false, &s(&temp.path().join("records")), &defs).unwrap();
        assert!(plan.installable, "{game}: {:?}", plan.reasons);
        let installed = reg.package_apply(plan.handle).unwrap();
        assert_eq!(installed.state, "committed");
        let expected = std::fs::read(root.join(format!("working/db/{file}.db"))).unwrap();
        let actual = std::fs::read(&db).unwrap();
        std::fs::write(root.join("activated-test.db"), &actual).unwrap();
        // Allocation/CRC and unused row bits can differ after structural edits;
        // every live field must match the author's saved content exactly.
        let a = tdb_core::TdbFile::parse(&actual).unwrap();
        let b = tdb_core::TdbFile::parse(&expected).unwrap();
        for table in &b.tables {
            let name = table.id.canonical(b.endian);
            let other = a.table_canonical(&name).unwrap();
            assert_eq!(table.current_records, other.current_records, "{game} {name}");
            for r in 0..usize::from(table.current_records) {
                for field in &table.fields {
                    let id = field.id.canonical(b.endian);
                    let target_field = other.fields.iter().find(|f|f.id.canonical(a.endian) == id).unwrap();
                    if field.bit_width <= 64 {
                        assert_eq!(a.read_u64(other,r as u32,target_field), b.read_u64(table,r as u32,field), "{game} {name}.{id} row {r}");
                    } else {
                        assert_eq!(a.read_inline_str(other,r as u32,target_field), b.read_inline_str(table,r as u32,field), "{game} {name}.{id} row {r}");
                    }
                }
            }
        }
        assert_eq!(reg.package_restore(&installed.record_dir).unwrap().state, "restored");
        assert!(std::fs::read(&db).unwrap() == before);
    }
}
