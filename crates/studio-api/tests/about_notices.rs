//! Receipt 46 — the About & Licences reader on archive layouts: a synthetic complete / broken archive root in a
//! temporary folder with spaces, and (ignored) a real extracted candidate named by `NHLMS_RELEASE_ROOT`.
use std::path::{Path, PathBuf};

use studio_api::about;

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn archive(tag: &str) -> Scratch {
    let root = std::env::temp_dir().join(format!("nhlms about {tag} {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write(&root, "BUILD-RECORD.json", r#"{"candidate":"rc-test","release_ready":false,"release_blockers":["Studio's MIT licence file: the copyright holder has not been recorded."]}"#);
    write(&root, "THIRD-PARTY-NOTICES.txt", "Status: INTERNAL REVIEW BUILD\nLicence text: licenses\\libspeex\\COPYING.\nfiles under licenses\\rust\\<name>-<version>\\.\n");
    write(&root, "licenses/libspeex/COPYING", "Copyright Xiph.Org");
    write(&root, "licenses/rust/zip-2.0.0/LICENSE", "MIT");
    write(&root, "helpers/nhlms-xma2-helper-notices/licenses/ffmpeg/COPYING.LGPLv2.1", "LGPL");
    Scratch(root)
}

#[test]
fn a_complete_internal_candidate_reads_offline_with_its_blockers() {
    let s = archive("complete");
    let a = about::read(Some(&s.0), "0.0.0");
    assert!(a.packaged && !a.release_ready, "{a:?}", a = a.problems);
    assert_eq!(a.candidate.as_deref(), Some("rc-test"));
    assert!(a.release_blockers[0].contains("copyright holder"));
    assert!(a.problems.is_empty(), "{:?}", a.problems);
    assert!(a.studio_licence_text.is_none() && a.studio_licence_spdx == "MIT");
    let files: Vec<&str> = a.licence_files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(files, ["licenses/libspeex/COPYING", "licenses/rust/zip-2.0.0/LICENSE", "helpers/nhlms-xma2-helper-notices/licenses/ffmpeg/COPYING.LGPLv2.1"]);
    assert_eq!(about::read_file(&s.0, "licenses/libspeex/COPYING").unwrap(), "Copyright Xiph.Org");
    assert_eq!(about::read_file(&s.0, "licenses\\libspeex\\COPYING").unwrap(), "Copyright Xiph.Org");
    // Only the listed files: nothing else in (or outside) the archive can be read through this command.
    for bad in ["BUILD-RECORD.json", "../secret.txt", "licenses/../BUILD-RECORD.json", "C:/Windows/win.ini", "licenses/libspeex"] {
        assert!(about::read_file(&s.0, bad).is_err(), "{bad} must be refused");
    }
}

#[test]
fn missing_packaged_files_are_named_by_path() {
    let s = archive("broken");
    std::fs::remove_dir_all(s.0.join("licenses/libspeex")).unwrap();
    std::fs::remove_dir_all(s.0.join("helpers")).unwrap();
    let a = about::read(Some(&s.0), "0.0.0");
    assert!(a.problems.iter().any(|p| p.contains("licenses/libspeex/COPYING") && p.contains("missing")), "{:?}", a.problems);
    assert!(a.problems.iter().any(|p| p.starts_with("Required folder missing") && p.contains("helpers")), "{:?}", a.problems);
    std::fs::remove_file(s.0.join("THIRD-PARTY-NOTICES.txt")).unwrap();
    let a = about::read(Some(&s.0), "0.0.0");
    assert!(a.notices.is_none() && a.problems.iter().any(|p| p.contains("THIRD-PARTY-NOTICES.txt")), "{:?}", a.problems);
    // A candidate that claims release-ready without Studio's licence is reported.
    write(&s.0, "BUILD-RECORD.json", r#"{"release_ready":true,"release_blockers":[]}"#);
    let a = about::read(Some(&s.0), "0.0.0");
    assert!(a.problems.iter().any(|p| p.contains("LICENSE")), "{:?}", a.problems);
}

#[test]
fn a_development_build_says_the_packaged_files_are_absent() {
    let dir = std::env::temp_dir().join(format!("nhlms about dev {}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let _g = Scratch(dir.clone());
    let a = about::read(Some(&dir), "0.0.0");
    assert!(!a.packaged && a.notices.is_none() && a.licence_files.is_empty());
    assert!(a.problems[0].contains("not running from an extracted release archive"), "{:?}", a.problems);
    let a = about::read(None, "0.0.0");
    assert!(a.problems[0].contains("could not resolve"));
}

#[test]
#[ignore = "needs an extracted candidate: NHLMS_RELEASE_ROOT=<root> (select with --ignored)"]
fn an_extracted_candidate_has_every_file_its_notices_name() {
    let root = PathBuf::from(std::env::var("NHLMS_RELEASE_ROOT").expect("NHLMS_RELEASE_ROOT (fails, never skips)"));
    let a = about::read(Some(&root), "0.0.0");
    assert!(a.packaged, "{:?}", a.problems);
    assert!(a.problems.is_empty(), "{:?}", a.problems);
    assert!(a.notices.as_deref().is_some_and(|n| n.contains("FFmpeg 8.1.3") && n.contains("WebView2 SDK")));
    assert!(a.licence_files.len() > 100, "{}", a.licence_files.len());
    for f in a.licence_files.iter().step_by(25) {
        assert!(!about::read_file(&root, &f.path).unwrap().is_empty(), "{}", f.path);
    }
    eprintln!("{}: ready {}, blockers {:?}, {} licence files", a.candidate.unwrap_or_default(), a.release_ready, a.release_blockers, a.licence_files.len());
}
