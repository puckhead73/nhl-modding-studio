//! **Retired / orphan head names** (ported from the operator-curated head-id
//! registry — see `defs/appearance/orphan-heads.toml`).
//!
//! Some `player_<id>.rx2` head assets ship on disk but are referenced by no bio
//! row in the title's DB — leftovers from an earlier roster whose player was
//! removed. The head census can't read a name for them (there's no player to
//! read it from), so the appearance sweep would file them as bare
//! `generic_<id>`. This table restores their real identity: a swept head whose
//! `(game, id)` matches an entry here is named for the player it modelled.
//!
//! IDs are **per-title** (a head id is assigned per roster), so every lookup is
//! scoped by the game token — a Legacy orphan id means nothing on NHL 11.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

/// The def, embedded at build time so the domain needs no runtime defs path.
const ORPHAN_HEADS_TOML: &str = include_str!("../../../defs/appearance/orphan-heads.toml");

#[derive(Deserialize)]
struct OrphanFile {
    #[serde(default)]
    head: Vec<OrphanRow>,
}

#[derive(Deserialize)]
struct OrphanRow {
    game: String,
    id: u64,
    last: String,
    first: String,
}

/// The real name behind an orphan head, ready for a catalog base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrphanName {
    /// Raw last name (display / reference).
    pub last: String,
    /// Raw first name.
    pub first: String,
}

impl OrphanName {
    /// The catalog base for this retired head: `{last}_{first}`, normalised the
    /// same way a live player's export slug is (lowercase ASCII, punctuation and
    /// spaces dropped) so `John-Michael Liles` → `liles_johnmichael`. There's no
    /// birthdate for a hand-named orphan, so the base carries name only.
    pub fn base(&self) -> String {
        format!(
            "{}_{}",
            tdb_core::dev_key::slug_ascii(&self.last),
            tdb_core::dev_key::slug_ascii(&self.first)
        )
    }
}

/// `(game, id)` → name index, parsed once from the embedded def.
fn index() -> &'static HashMap<(String, u64), OrphanName> {
    static INDEX: OnceLock<HashMap<(String, u64), OrphanName>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let file: OrphanFile = toml::from_str(ORPHAN_HEADS_TOML)
            .expect("defs/appearance/orphan-heads.toml is malformed");
        file.head
            .into_iter()
            .map(|r| {
                (
                    (r.game, r.id),
                    OrphanName {
                        last: r.last,
                        first: r.first,
                    },
                )
            })
            .collect()
    })
}

/// The real name for an orphan head id in `game`, if this title has one on
/// record. `game` is the workspace game token (`nhllegacy`, …).
pub fn orphan_head_name(game: &str, id: u64) -> Option<&'static OrphanName> {
    index().get(&(game.to_string(), id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_legacy_orphans_resolve() {
        let hull = orphan_head_name("nhllegacy", 8323).expect("Hull");
        assert_eq!((hull.last.as_str(), hull.first.as_str()), ("Hull", "Brett"));
        assert_eq!(hull.base(), "hull_brett");
        // Punctuation/spacing normalised like the live slug.
        assert_eq!(
            orphan_head_name("nhllegacy", 532).unwrap().base(),
            "liles_johnmichael"
        );
        assert_eq!(orphan_head_name("nhllegacy", 1066).unwrap().first, "Steve");
    }

    #[test]
    fn lookups_are_title_scoped() {
        // Legacy ids carry no meaning on another title.
        assert!(orphan_head_name("nhl11", 8323).is_none());
        assert!(orphan_head_name("nhl15", 66).is_none());
        // An id with no entry is None even on the right title.
        assert!(orphan_head_name("nhllegacy", 999_999).is_none());
    }

    #[test]
    fn every_row_parses_and_is_named() {
        // The whole embedded def is well-formed and no row is blank.
        for ((game, id), name) in index() {
            assert!(!game.is_empty(), "row {id} has no game");
            assert!(
                !name.last.is_empty() && !name.first.is_empty(),
                "row {id} has a blank name"
            );
            assert!(!name.base().is_empty(), "row {id} produced an empty base");
        }
        assert!(index().len() >= 30, "expected the full orphan set");
    }
}
