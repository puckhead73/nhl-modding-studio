//! `content_header::ensure_content_header` — sidecar maintenance for recomp/Xenia content trees.
//! A save container written into such a tree without its `Headers/<type>/<NAME>.header` sidecar is
//! invisible to the game's save list (verified against a real NHL Legacy Recomp tree).

use std::fs;
use std::path::{Path, PathBuf};
use studio_domain::content_header::ensure_content_header;

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-content-header-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Build `<root>/<profile>/<title>/00000001/<name>/<name>` (+ optional Headers dir), returning the
/// save file path.
fn make_tree(root: &Path, profile: &str, title: &str, name: &str, with_headers: bool) -> PathBuf {
    let save_dir = root.join(profile).join(title).join("00000001").join(name);
    fs::create_dir_all(&save_dir).unwrap();
    let save_file = save_dir.join(name);
    fs::write(&save_file, b"container bytes irrelevant to the sidecar").unwrap();
    if with_headers {
        fs::create_dir_all(root.join(profile).join(title).join("Headers").join("00000001")).unwrap();
    }
    save_file
}

#[test]
fn synthesizes_sidecar_in_header_indexed_tree() {
    let tmp = Tmp::new("synth");
    let save = make_tree(&tmp.0, "B13EBABEBABEBABE", "454109EC", "ROSTER 20260723000649", true);

    let created = ensure_content_header(&save).unwrap().expect("sidecar created");
    assert!(created.ends_with("Headers/00000001/ROSTER 20260723000649.header"));
    let h = fs::read(&created).unwrap();
    assert_eq!(h.len(), 0x148);
    assert_eq!(&h[0x00..0x04], &[0, 0, 0, 1]); // content type: save
    assert_eq!(&h[0x108..0x11D], b"ROSTER 20260723000649");
    assert_eq!(&h[0x138..0x140], &[0xB1, 0x3E, 0xBA, 0xBE, 0xBA, 0xBE, 0xBA, 0xBE]);
    assert_eq!(&h[0x140..0x144], &[0x45, 0x41, 0x09, 0xEC]);

    // Second call: sidecar exists → no-op.
    assert!(ensure_content_header(&save).unwrap().is_none());
}

#[test]
fn clones_same_type_template_and_patches_only_the_name() {
    let tmp = Tmp::new("template");
    let save = make_tree(&tmp.0, "B13EBABEBABEBABE", "454109EC", "ROSTER 20260723000649", true);

    // A stale same-type sidecar with game-authored display strings (and stale tail garbage).
    let mut template = vec![0u8; 0x148];
    template[0x00..0x08].copy_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1]);
    template[0x08..0x16].copy_from_slice(b"\0R\0O\0S\0T\0E\0R\0\x31"); // "ROSTER1" UTF-16BE
    let old_name = b"ROSTER 20260702231158\x001134"; // stale tail after the NUL, like real files
    template[0x108..0x108 + old_name.len()].copy_from_slice(old_name);
    template[0x138..0x148]
        .copy_from_slice(&[0xB1, 0x3E, 0xBA, 0xBE, 0xBA, 0xBE, 0xBA, 0xBE, 0x45, 0x41, 0x09, 0xEC, 0, 0, 0, 0]);
    let headers = tmp.0.join("B13EBABEBABEBABE/454109EC/Headers/00000001");
    fs::write(headers.join("ROSTER 20260702231158.header"), &template).unwrap();
    // A different-type sidecar that must NOT be used as the template.
    fs::write(headers.join("BUILDYOURAI 20260723020630.header"), vec![0u8; 0x148]).unwrap();

    let created = ensure_content_header(&save).unwrap().expect("sidecar created");
    let h = fs::read(&created).unwrap();
    // Display strings preserved from the same-type template…
    assert_eq!(&h[0x08..0x16], b"\0R\0O\0S\0T\0E\0R\0\x31");
    // …name patched with the stale tail zeroed…
    assert_eq!(&h[0x108..0x11D], b"ROSTER 20260723000649");
    assert!(h[0x11D..0x138].iter().all(|&b| b == 0));
    // …ids carried through.
    assert_eq!(&h[0x140..0x144], &[0x45, 0x41, 0x09, 0xEC]);
}

#[test]
fn noop_outside_header_indexed_trees() {
    // Content-shaped tree but no Headers dir (plain Xenia layout): stays untouched.
    let tmp = Tmp::new("noheaders");
    let save = make_tree(&tmp.0, "B13EBABEBABEBABE", "454109EC", "ROSTER 20260723000649", false);
    assert!(ensure_content_header(&save).unwrap().is_none());
    assert!(!tmp.0.join("B13EBABEBABEBABE/454109EC/Headers").exists(), "never invents the layout");

    // Not a content tree at all (e.g. a PS3 SYS-DATA path): no-op.
    let tmp2 = Tmp::new("ps3");
    let dir = tmp2.0.join("BLUS315400200 - ROSTER");
    fs::create_dir_all(&dir).unwrap();
    let f = dir.join("SYS-DATA");
    fs::write(&f, b"x").unwrap();
    assert!(ensure_content_header(&f).unwrap().is_none());
}
