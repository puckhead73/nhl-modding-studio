//! NHL07 (Xbox 360) save containers verified read-only. Unlike the nhl14+ PS3 roster save — a
//! record-index-aligned mirror of `nhlng.db` the editor writes through — every NHL07 console save is
//! a **container version the current codec does not support** (ROSTER1 / DYNASTY1 are `v3`; USER
//! PROFILE1 parses but exposes no inner db). So on NHL07 the sole editable surface is the loose
//! `NHL07NG.db`, and the three saves are surfaced strictly read-only. This pins that: if a future
//! codec learns to extract them, this test flags that the roster-mirror + write path now needs
//! wiring (and re-verification) for gen6.

use std::path::{Path, PathBuf};

use tdb_savedata::SaveContainer;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

/// None of NHL07's Xbox saves yield an editable inner db today, so all classify non-writable. The
/// loose `NHL07NG.db` (exercised by the field-profile tests) is the editable surface.
#[test]
fn nhl07_console_saves_are_read_only() {
    for name in ["ROSTER1/ROSTER1", "DYNASTY1/DYNASTY1", "USER PROFILE1/USER PROFILE1"] {
        let bytes = std::fs::read(repo(&format!("anchors/xbox360/nhl07/savefiles/{name}"))).unwrap();
        let extractable = SaveContainer::parse(&bytes).ok().and_then(|c| c.extract_db().ok()).is_some();
        assert!(
            !extractable,
            "{name}: NHL07 saves are read-only today — an extractable inner db here means the \
             roster-mirror/write path must be wired and re-verified for gen6"
        );
    }
}
