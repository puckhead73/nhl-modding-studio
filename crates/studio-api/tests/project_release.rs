use std::{fs, path::{Path, PathBuf}};
use studio_api::{Registry, dto::ProjectDetailsDto};
use studio_domain::{BaselineStore, Platform, FieldValue, project_store::ProjectStore, mod_package};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into() }
fn inventory() -> BaselineStore { BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap() }

#[test]
fn change_project_folder_preserves_sources_edits_and_save_time() {
    let temp = tempfile::tempdir().unwrap();
    let reg = Registry::default(); let inv = inventory(); let defs = s(&repo("defs"));
    let original = temp.path().join("Original");
    let target = temp.path().join("New storage/Project");
    let opened = reg.project_create_fresh(&s(&original), "nhl07", Platform::Xbox, "", None, None, &inv, &defs).unwrap();
    let id = opened.summary.project_id;
    assert!(opened.project.last_saved.is_none());
    assert!(reg.project_save(id).unwrap().nothing_to_save);
    assert!(reg.project_info(id).unwrap().last_saved.is_some());
    reg.set_jersey_number(id, 14, 613, 90).unwrap();
    assert!(reg.project_copy_to(id, &s(&target)).is_err());
    reg.project_save(id).unwrap();
    fs::create_dir_all(original.join("Art sources")).unwrap();
    fs::write(original.join("Art sources/notes.txt"), b"Keep me").unwrap();
    assert!(reg.project_copy_to(id, &s(&original.join("Nested"))).is_err());
    let expected = fs::read(original.join("working/db/NHL07NG.db")).unwrap();
    reg.project_copy_to(id, &s(&target)).unwrap();
    assert!(reg.project_copy_to(id, &s(&target)).is_err());
    assert_eq!(fs::read(original.join("working/db/NHL07NG.db")).unwrap(), expected);
    assert_eq!(fs::read(target.join("working/db/NHL07NG.db")).unwrap(), expected);
    assert_eq!(fs::read(target.join("Art sources/notes.txt")).unwrap(), b"Keep me");
    assert!(reg.close_project(id));
    let reopened = reg.project_open(&s(&target), &defs).unwrap();
    assert!(reopened.project.last_saved.is_some());
    assert!(!reg.project_overview(reopened.summary.project_id).unwrap().changes.is_empty());
    reg.set_jersey_number(reopened.summary.project_id, 14, 613, 91).unwrap();
    assert!(reg.project_save(reopened.summary.project_id).unwrap().saved);
    assert_eq!(fs::read(original.join("working/db/NHL07NG.db")).unwrap(), expected);
    reg.close_project(reopened.summary.project_id);
}

#[test]
fn nhl07_build_and_archive_round_trip_preserve_project_and_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let reg = Registry::default(); let inv = inventory(); let defs = s(&repo("defs"));
    let dir = temp.path().join("Project");
    let open = reg.project_create_fresh(&s(&dir), "nhl07", Platform::Xbox, "", None, None, &inv, &defs).unwrap();
    let id = open.summary.project_id;
    reg.project_set_details(id, ProjectDetailsDto { name: "QA - NHL07".into(), version: "1.2".into(), author: "Mod Team".into(), description: "Release test".into() }).unwrap();
    assert!(reg.project_build(id, &s(temp.path()), Some(&inv), &defs).is_err());
    reg.set_jersey_number(id, 14, 613, 90).unwrap();
    assert!(reg.project_build(id, &s(temp.path()), Some(&inv), &defs).is_err());
    assert!(reg.project_archive(id, &s(temp.path())).is_err());
    reg.project_save(id).unwrap();
    let build = reg.project_build(id, &s(temp.path()), Some(&inv), &defs).unwrap();
    assert_eq!(build.packages.len(), 1);
    assert!(build.packages[0].path.ends_with("QA - NHL07_xbox360_nhl07.nhlmod"));
    let inspected = mod_package::inspect_package(Path::new(&build.packages[0].path), &mod_package::Limits::default()).unwrap();
    assert_eq!(inspected.manifest.author, "Mod Team");
    assert_eq!(inspected.manifest.version, "1.2");
    assert_eq!(inspected.manifest.targets.len(), 1);
    assert_eq!(inspected.manifest.targets[0].platform, "xbox");
    assert!(!inspected.edits.cells.is_empty());
    fs::write(temp.path().join("Builds/broken.nhlmod"), b"unfinished download").unwrap();
    let library = reg.mod_library_scan(&s(&temp.path().join("Builds"))).unwrap();
    assert_eq!(library.mods.len(), 1);
    assert_eq!(library.mods[0].author, "Mod Team");
    assert_eq!(library.mods[0].package_id, inspected.manifest.package_id);
    assert_eq!(library.mods[0].game_id, "nhl07");
    assert_eq!(library.mods[0].platforms, vec!["xbox"]);
    assert_eq!(library.warnings.len(), 1);
    let original_package = fs::read(&build.packages[0].path).unwrap();
    let next = reg.project_build(id, &s(temp.path()), Some(&inv), &defs).unwrap();
    assert_ne!(next.packages[0].path, build.packages[0].path);
    assert_eq!(fs::read(&build.packages[0].path).unwrap(), original_package);
    // Include arbitrary developer source files too, not just known runtime assets.
    fs::create_dir_all(dir.join("Art sources")).unwrap();
    fs::write(dir.join("Art sources/portrait.psd"), b"developer source payload").unwrap();
    let archive = reg.project_archive(id, &s(temp.path())).unwrap();
    let mut zip = zip::ZipArchive::new(fs::File::open(&archive.path).unwrap()).unwrap();
    assert!(zip.by_name("Art sources/portrait.psd").is_ok());
    assert!(zip.by_name("nhl-studio-project.lock").is_err());
    let restored = temp.path().join("Restored");
    drop(zip);
    let preview = reg.project_archive_preview(&archive.path).unwrap();
    assert_eq!(preview.game_id, "nhl07");
    assert_eq!(preview.platform, "xbox");
    assert!(reg.project_import_archive(&archive.path, &s(&temp.path().join("bad-source")), Some(&s(&temp.path().join("missing-source")))).is_err());
    assert!(!temp.path().join("bad-source").exists());
    // Corrupt an immutable baseline while keeping a valid ZIP: manifest identities must catch it.
    let corrupt = temp.path().join("changed-baseline.nhlproject");
    {
        use std::io::{Read, Write};
        let baseline = ProjectStore::open(&dir).unwrap().manifest.source.components[0].baseline.clone();
        let mut input = zip::ZipArchive::new(fs::File::open(&archive.path).unwrap()).unwrap();
        let mut output = zip::ZipWriter::new(fs::File::create(&corrupt).unwrap());
        for i in 0..input.len() {
            let mut entry = input.by_index(i).unwrap();
            let name = entry.name().to_string();
            let mut data = vec![]; entry.read_to_end(&mut data).unwrap();
            if name == baseline { data[0] ^= 1; }
            output.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            output.write_all(&data).unwrap();
        }
        output.finish().unwrap();
    }
    let rejected = temp.path().join("Rejected");
    assert!(reg.project_import_archive(&s(&corrupt), &s(&rejected), None).unwrap_err().to_string().contains("verification failed"));
    assert!(!rejected.exists());
    reg.project_import_archive(&archive.path, &s(&restored), None).unwrap();
    assert!(reg.project_import_archive(&archive.path, &s(&restored), None).is_err());
    assert_eq!(fs::read(restored.join("Art sources/portrait.psd")).unwrap(), b"developer source payload");
    assert!(ProjectStore::open(&restored).unwrap().manifest.destinations.install_dir.is_none());
    let source_folder = temp.path().join("User game source");
    fs::create_dir(&source_folder).unwrap();
    let relinked = temp.path().join("Relinked");
    reg.project_import_archive(&archive.path, &s(&relinked), Some(&s(&source_folder))).unwrap();
    assert_eq!(ProjectStore::open(&relinked).unwrap().manifest.destinations.assets_dir, Some(s(&source_folder)));
    reg.close_project(id);
    let reopened = reg.project_open(&s(&restored), &defs).unwrap();
    let review = reg.project_overview(reopened.summary.project_id).unwrap();
    assert_eq!(review.details.author, "Mod Team");
    assert!(!review.changes.is_empty());
    assert_eq!(fs::read(restored.join("Art sources/portrait.psd")).unwrap(), b"developer source payload");
    // The recipient can continue editing and saving the copied project.
    reg.set_jersey_number(reopened.summary.project_id, 14, 613, 91).unwrap();
    assert!(reg.project_save(reopened.summary.project_id).unwrap().saved);
    let rebuilt = reg.project_build(reopened.summary.project_id, &s(temp.path()), Some(&inv), &defs).unwrap();
    assert_eq!(rebuilt.packages.len(), 1);
    reg.close_project(reopened.summary.project_id);
}

#[test]
fn archive_import_rejects_unsafe_or_damaged_projects_without_publishing() {
    use std::io::Write;
    let temp = tempfile::tempdir().unwrap();
    let reg = Registry::default();
    for (index, name) in ["../escape", "C:/escape", "nested/../escape", "folder\\escape", "file:stream", "CON.txt", "working/.tdb-txn/ACTIVE"].iter().enumerate() {
        let path = temp.path().join(format!("unsafe{index}.nhlproject"));
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"payload").unwrap(); zip.finish().unwrap();
        let dest = temp.path().join(format!("result{index}"));
        assert!(reg.project_import_archive(&s(&path), &s(&dest), None).is_err());
        assert!(!dest.exists());
    }
    let damaged = temp.path().join("damaged.nhlproject");
    fs::write(&damaged, b"broken archive").unwrap();
    assert!(reg.project_archive_preview(&s(&damaged)).is_err());
    assert!(reg.project_import_archive(&s(&damaged), &s(&temp.path().join("damaged")), None).is_err());
    assert!(!temp.path().join("escape").exists());
}

#[test]
fn nhl15_builds_separate_ps3_and_360_packages_and_never_ps4() {
    let temp = tempfile::tempdir().unwrap();
    let reg = Registry::default(); let inv = inventory(); let defs = s(&repo("defs"));
    let open = reg.project_create_fresh(&s(&temp.path().join("Project")), "nhl15", Platform::Ps3, "", None, None, &inv, &defs).unwrap();
    let id = open.summary.project_id;
    reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(5)).unwrap();
    if !reg.summary(id).unwrap().dirty { reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(4)).unwrap(); }
    reg.project_save(id).unwrap();
    let build = reg.project_build(id, &s(temp.path()), Some(&inv), &defs).unwrap();
    assert_eq!(build.packages.len(), 2);
    let mut targets = vec![];
    for p in build.packages {
        let package = mod_package::inspect_package(Path::new(&p.path), &mod_package::Limits::default()).unwrap();
        assert_eq!(package.manifest.targets.len(), 1);
        assert_eq!(package.manifest.author_platform, "ps3");
        targets.push(package.manifest.targets[0].platform.clone());
    }
    targets.sort(); assert_eq!(targets, vec!["ps3", "xbox"]);
    reg.close_project(id);
}

#[test]
fn unpackageable_content_is_not_silently_omitted() {
    let temp = tempfile::tempdir().unwrap();
    let reg = Registry::default(); let inv = inventory(); let defs = s(&repo("defs"));
    let dir = temp.path().join("Project");
    let open = reg.project_create_fresh(&s(&dir), "nhl07", Platform::Xbox, "", None, None, &inv, &defs).unwrap();
    reg.close_project(open.summary.project_id);
    let mut project = ProjectStore::open(&dir).unwrap();
    fs::create_dir_all(project.overlay_dir().join("test")).unwrap();
    fs::write(project.overlay_dir().join("test/unknown.bin"), b"asset").unwrap();
    project.record_staged_file("test/unknown.bin", "test", None).unwrap();
    let open = reg.project_open(&s(&dir), &defs).unwrap();
    let error = reg.project_build(open.summary.project_id, &s(temp.path()), Some(&inv), &defs).err().unwrap().to_string();
    assert!(error.contains("cannot be packaged"), "{error}");
    assert!(!temp.path().join("Builds").exists());
    reg.close_project(open.summary.project_id);
}
