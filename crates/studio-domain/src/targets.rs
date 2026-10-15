//! The 27 shipping targets (ONB-002): one row per `(game id, platform)` the Studio ships for, with
//! the file names each target's native database set uses. This is the backend's own list — the
//! frontend catalog (`app/src/lib/catalog.ts`) is the UI's — and the two are kept equal by
//! `tests/shipping_targets.rs` in `studio-api`.
//!
//! NHL 07 on PS3 is a research copy, not a shipping target, and there is no Xbox One target
//! (CLAUDE.md, accepted scope).

use crate::Platform;

/// One shipping target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShippingTarget {
    pub game_id: &'static str,
    pub platform: Platform,
    /// Display title ("NHL 14").
    pub title: &'static str,
    /// The main database's file name (`nhlng.db`, `NHL07NG.db`, `nhl_game.db`).
    pub main_db: &'static str,
    /// Its schema metadata sibling.
    pub meta: &'static str,
    /// The localization dictionary file name.
    pub dictionary: &'static str,
    /// The dictionary's schema metadata sibling.
    pub dictionary_meta: &'static str,
}

const fn gen7(game_id: &'static str, platform: Platform, title: &'static str) -> ShippingTarget {
    ShippingTarget {
        game_id,
        platform,
        title,
        main_db: "nhlng.db",
        meta: "nhlng-meta.xml",
        dictionary: "nhl_eng_us.db",
        dictionary_meta: "nhl_eng_us-meta.xml",
    }
}

const fn ps4(game_id: &'static str, title: &'static str) -> ShippingTarget {
    ShippingTarget {
        game_id,
        platform: Platform::Ps4,
        title,
        main_db: "nhl_game.db",
        meta: "nhl_game-meta.xml",
        dictionary: "eng_us.db",
        dictionary_meta: "eng_us-meta.xml",
    }
}

/// Every shipping target, in catalog order: Xbox 360 NHL 07–15 + Legacy (10), PS3 NHL 08–15 +
/// Legacy (9), PS4 NHL 15–21 (7), NHL Legacy Recompiled (1).
pub const SHIPPING_TARGETS: &[ShippingTarget] = &[
    ShippingTarget {
        game_id: "nhl07",
        platform: Platform::Xbox,
        title: "NHL 07",
        main_db: "NHL07NG.db",
        meta: "NHL07NG-meta.xml",
        dictionary: "nhl_eng_us.db",
        dictionary_meta: "nhl_eng_us-meta.xml",
    },
    gen7("nhl08", Platform::Xbox, "NHL 08"),
    gen7("nhl09", Platform::Xbox, "NHL 09"),
    gen7("nhl10", Platform::Xbox, "NHL 10"),
    gen7("nhl11", Platform::Xbox, "NHL 11"),
    gen7("nhl12", Platform::Xbox, "NHL 12"),
    gen7("nhl13", Platform::Xbox, "NHL 13"),
    gen7("nhl14", Platform::Xbox, "NHL 14"),
    gen7("nhl15", Platform::Xbox, "NHL 15"),
    gen7("nhllegacy", Platform::Xbox, "NHL Legacy"),
    gen7("nhl08", Platform::Ps3, "NHL 08"),
    gen7("nhl09", Platform::Ps3, "NHL 09"),
    gen7("nhl10", Platform::Ps3, "NHL 10"),
    gen7("nhl11", Platform::Ps3, "NHL 11"),
    gen7("nhl12", Platform::Ps3, "NHL 12"),
    gen7("nhl13", Platform::Ps3, "NHL 13"),
    gen7("nhl14", Platform::Ps3, "NHL 14"),
    gen7("nhl15", Platform::Ps3, "NHL 15"),
    gen7("nhllegacy", Platform::Ps3, "NHL Legacy"),
    ps4("nhl15", "NHL 15"),
    ps4("nhl16", "NHL 16"),
    ps4("nhl17", "NHL 17"),
    ps4("nhl18", "NHL 18"),
    ps4("nhl19", "NHL 19"),
    ps4("nhl20", "NHL 20"),
    ps4("nhl21", "NHL 21"),
    gen7("legacyrecomp", Platform::Recomp, "NHL Legacy Recompiled"),
];

/// The shipping target for a `(game id, platform)`, if it ships.
pub fn shipping_target(game_id: &str, platform: Platform) -> Option<&'static ShippingTarget> {
    SHIPPING_TARGETS
        .iter()
        .find(|t| t.game_id == game_id && t.platform == platform)
}

/// The anchor directory name of a platform (`anchors/<this>/<game>/…`).
pub fn anchor_platform_dir(platform: Platform) -> &'static str {
    match platform {
        Platform::Ps3 => "ps3",
        Platform::Xbox => "xbox360",
        Platform::Ps4 => "ps4",
        // Recompiled derives from the Xbox 360 Legacy anchors (handoff R128).
        Platform::Recomp => "xbox360",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_seven_targets_and_no_research_or_xbox_one_rows() {
        assert_eq!(SHIPPING_TARGETS.len(), 27);
        assert!(shipping_target("nhl07", Platform::Ps3).is_none());
        assert!(shipping_target("nhl07", Platform::Xbox).is_some());
        assert!(shipping_target("nhl16", Platform::Ps3).is_none());
        assert_eq!(
            SHIPPING_TARGETS.iter().filter(|t| t.platform == Platform::Xbox).count(),
            10
        );
        assert_eq!(SHIPPING_TARGETS.iter().filter(|t| t.platform == Platform::Ps3).count(), 9);
        assert_eq!(SHIPPING_TARGETS.iter().filter(|t| t.platform == Platform::Ps4).count(), 7);
        assert_eq!(SHIPPING_TARGETS.iter().filter(|t| t.platform == Platform::Recomp).count(), 1);
        let mut seen = std::collections::HashSet::new();
        for t in SHIPPING_TARGETS {
            assert!(seen.insert((t.game_id, t.platform.as_str())), "duplicate {} {:?}", t.game_id, t.platform);
        }
    }
}
