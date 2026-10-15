//! Isolate packed-disc runtime behavior from the extracted loose-asset layout.
use std::{path::Path, sync::atomic::AtomicBool};

#[test]
#[ignore = "copies NHL14 ISO into a fresh owned packed-only baseline; never modifies the ISO"]
fn packed_only_baseline() {
    let iso = Path::new("X:/Games/Emulation/Roms/XBOX360/NHL 14/NHL 14 (USA, Europe) (En,Fr,De,Sv,Fi,Ru,Cs).iso");
    let root = Path::new("H:/TEMP/nhl-studio-overnight-20261003/nhl14/packed-only-baseline");
    assert!(!root.exists());
    let receipt = studio_domain::disc_extract::extract(iso, root, &AtomicBool::new(false), &mut |_, _| {}).unwrap();
    assert!(root.join("default.xex").is_file());
    assert!(!root.join("rendering").exists(), "keep the native packed layout");
    std::fs::write(root.with_extension("json"), serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}
