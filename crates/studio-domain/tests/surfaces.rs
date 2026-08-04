//! Surface-capability probing locked against representative anchors (Phase 1B). Mirrors the field-
//! level `capabilities.rs` tests but at the *surface* granularity the UI gates tabs on. Expectations
//! are grounded in the committed schema snapshots (`docs/audits/data/schema/`): NHL 07 (gen6) is the
//! thin shape, NHL 10 sits mid-era, NHL 12 / Legacy / PS4-21 carry the full set. Asset surfaces stay
//! `Disabled` everywhere (authoring is a later phase — never promoted here).

use std::path::{Path, PathBuf};

use studio_domain::{Capabilities, SurfaceCapabilities, SurfaceState};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_relations::RosterModel;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn probe(rel_db: &str, rel_meta: &str) -> SurfaceCapabilities {
    let bytes = std::fs::read(repo(rel_db)).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(rel_meta)).unwrap()).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let caps = Capabilities::probe(&db, &meta);
    SurfaceCapabilities::probe(&db, &meta, &model, &caps)
}

use SurfaceState::*;

/// Asset families never leave `Disabled` in Phase 1 (rule 12 — no asset promotion).
fn assert_assets_disabled(s: &SurfaceCapabilities) {
    assert_eq!(s.portraits.state, Disabled);
    assert_eq!(s.logos.state, Disabled);
    assert_eq!(s.heads.state, Disabled);
    assert_eq!(s.audio.state, Disabled);
    // Every Disabled/NotApplicable verdict carries a human reason.
    for c in [s.portraits, s.logos, s.heads, s.audio, s.player_equipment] {
        if c.state == Disabled || c.state == NotApplicable {
            assert!(c.reason.is_some(), "disabled/absent surface must explain itself");
        }
    }
}

/// NHL 07 (gen6): the thin shape. Its arena table carries no `arenaname`, it ships no draft-pick,
/// jersey, or equipment tables, and it has no trait system — every one of those omits its tab.
#[test]
fn nhl07_gen6_omits_the_absent_surfaces() {
    let s = probe("anchors/xbox360/nhl07/databases/NHL07NG.db", "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml");
    // Present, universal.
    assert_eq!(s.players.state, Editable);
    assert_eq!(s.teams.state, Editable);
    assert_eq!(s.attributes.state, Editable);
    assert_eq!(s.contracts.state, Editable);
    assert_eq!(s.lines.state, Editable);
    assert_eq!(s.captaincy.state, Editable);
    assert_eq!(s.strategies.state, Editable);
    // Absent on gen6 → NotApplicable (tabs omitted, not empty).
    assert_eq!(s.arenas.state, NotApplicable, "NHL07 StEO has no arenaname field");
    assert_eq!(s.draft_picks.state, NotApplicable, "NHL07 ships no qHav");
    assert_eq!(s.jerseys.state, NotApplicable, "NHL07 ships no xjgw");
    assert_eq!(s.team_equipment.state, NotApplicable, "NHL07 ships no hHbv");
    assert_eq!(s.player_equipment.state, NotApplicable, "NHL07 ships no xmja/fMVl");
    assert_eq!(s.traits.state, NotApplicable, "NHL07 has no trait slots");
    assert_assets_disabled(&s);
}

/// NHL 10: mid-era. Arena table + name present (the Arenas list works; the team→arena *link* is a
/// separate field-level gate), team equipment present, but no draft-pick or jersey tables and no
/// trait system yet. Per-player equipment tables exist → Disabled (no editor built), not omitted.
#[test]
fn nhl10_mid_era_shape() {
    let s = probe("anchors/xbox360/nhl10/databases/nhlng.db", "anchors/xbox360/nhl10/databases/nhlng-meta.xml");
    assert_eq!(s.arenas.state, Readable, "NHL10 StEO carries arenaname");
    assert_eq!(s.team_equipment.state, Editable);
    assert_eq!(s.draft_picks.state, NotApplicable, "NHL10 ships no qHav");
    assert_eq!(s.jerseys.state, NotApplicable, "NHL10 ships no xjgw");
    assert_eq!(s.traits.state, NotApplicable, "NHL10 has no trait slots");
    assert_eq!(s.player_equipment.state, Disabled, "xmja/fMVl exist but no editor (Phase 6)");
    assert_assets_disabled(&s);
}

/// NHL 12: draft picks + jerseys + traits have arrived — the full presentation set is available.
#[test]
fn nhl12_has_the_full_set() {
    let s = probe("anchors/ps3/nhl12/databases/nhlng.db", "anchors/ps3/nhl12/databases/nhlng-meta.xml");
    assert_eq!(s.draft_picks.state, Editable);
    assert_eq!(s.jerseys.state, Editable);
    assert_eq!(s.traits.state, Editable);
    assert_eq!(s.arenas.state, Readable);
    assert_eq!(s.player_equipment.state, Disabled);
    assert_assets_disabled(&s);
}

/// NHL Legacy (PS3) and PS4 NHL 21: every schema-driven surface is available.
#[test]
fn legacy_and_ps4_carry_every_surface() {
    for (db, meta) in [
        ("anchors/ps3/nhllegacy/databases/nhlng.db", "anchors/ps3/nhllegacy/databases/nhlng-meta.xml"),
        ("anchors/ps4/nhl21/databases/nhl_game.db", "anchors/ps4/nhl21/databases/nhl_game-meta.xml"),
    ] {
        let s = probe(db, meta);
        for (name, st) in [
            ("players", s.players.state),
            ("teams", s.teams.state),
            ("arenas", s.arenas.state),
            ("attributes", s.attributes.state),
            ("traits", s.traits.state),
            ("lines", s.lines.state),
            ("strategies", s.strategies.state),
            ("captaincy", s.captaincy.state),
            ("contracts", s.contracts.state),
            ("draft_picks", s.draft_picks.state),
            ("jerseys", s.jerseys.state),
            ("team_equipment", s.team_equipment.state),
        ] {
            assert_ne!(st, NotApplicable, "{db}: {name} should be available");
        }
        assert_eq!(s.player_equipment.state, Disabled, "{db}: equipment tables present, no editor yet");
        assert_assets_disabled(&s);
    }
}
