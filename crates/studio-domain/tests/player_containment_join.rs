//! Phase 3 (22 Sep 2026) — the gen7 portrait / head containment of research 2026-09-21 pm handoff
//! R53 §1 reproduced by Studio's own resolver on the real Xbox 360 / PS3 trees joined to the
//! committed anchors: bespoke counts and their file presence, the pool population, the
//! `hasportrait` × file cross-tab, the unused shipped pools, and NHL 09's "shipped without asset"
//! names. Read-only; a missing tree fails, never skips.

use std::path::{Path, PathBuf};

use studio_domain::player_containment::{bio_rows, containment_contract, player_containment};
use studio_domain::Platform;
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

struct Expect {
    game: &'static str,
    platform: Platform,
    anchor_dir: &'static str,
    root: &'static str,
    players: usize,
    bespoke: usize,
    bespoke_head: usize,
    bespoke_portrait: usize,
    pool: usize,
    flag1_file1: usize,
    flag1_file0: usize,
    flag0_file1: usize,
    flag0_file0: usize,
    unused_portraits: usize,
    unused_heads: usize,
}

const X360: &str = r"X:\Games\Emulation\Roms\XBOX360";
const PS3: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3";

/// R53 §1, transcribed (players / bespoke / bespoke head file / bespoke portrait file / non-bespoke = R53 "players_other" (Legacy: 5 385 pool + 1 other) /
/// flag cross-tab / unused portraits / unused heads).
const TITLES: &[Expect] = &[
    Expect { game: "nhl09", platform: Platform::Xbox, anchor_dir: "xbox360", root: r"NHL 09\NHL 09 (USA)", players: 3968, bespoke: 256, bespoke_head: 251, bespoke_portrait: 251, pool: 3712, flag1_file1: 2705, flag1_file0: 0, flag0_file1: 1, flag0_file0: 1262, unused_portraits: 214, unused_heads: 129 },
    Expect { game: "nhl13", platform: Platform::Xbox, anchor_dir: "xbox360", root: r"NHL 13\NHL 13", players: 5412, bespoke: 268, bespoke_head: 268, bespoke_portrait: 267, pool: 5144, flag1_file1: 4298, flag1_file0: 0, flag0_file1: 93, flag0_file0: 1021, unused_portraits: 2433, unused_heads: 375 },
    Expect { game: "nhllegacy", platform: Platform::Xbox, anchor_dir: "xbox360", root: r"NHL Legacy\NHL Legacy", players: 5632, bespoke: 246, bespoke_head: 246, bespoke_portrait: 246, pool: 5386, flag1_file1: 4376, flag1_file0: 2, flag0_file1: 127, flag0_file0: 1127, unused_portraits: 4724, unused_heads: 457 },
    Expect { game: "nhllegacy", platform: Platform::Ps3, anchor_dir: "ps3", root: r"NHL Legacy - Vanilla\PS3_GAME\USRDIR", players: 5632, bespoke: 246, bespoke_head: 246, bespoke_portrait: 246, pool: 5386, flag1_file1: 4376, flag1_file0: 2, flag0_file1: 127, flag0_file0: 1127, unused_portraits: 4724, unused_heads: 457 },
];

#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn the_containment_cross_tab_reproduces_the_research_on_the_real_trees() {
    for t in TITLES {
        let tag = format!("{} {}", t.game, t.platform.as_str());
        let root = Path::new(if t.platform == Platform::Ps3 { PS3 } else { X360 }).join(t.root);
        assert!(root.is_dir(), "{tag}: {} missing (fails, never skips)", root.display());
        let base = repo(&format!("anchors/{}/{}/databases", t.anchor_dir, t.game));
        let bytes = std::fs::read(base.join("nhlng.db")).unwrap_or_else(|e| panic!("{tag}: {e}"));
        let db = TdbFile::parse(&bytes).expect("anchor parses");
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(base.join("nhlng-meta.xml")).unwrap()).expect("meta");
        let (rows, has_flag) = bio_rows(&db, &meta).expect("bio rows");
        assert!(has_flag, "{tag}: the bio table carries `hasportrait`");
        let v = player_containment(&rows, has_flag, containment_contract(Some(t.game), t.platform), Some(&root.join("fe")), Some(&root.join("rendering")));
        let s = &v.summary;
        eprintln!(
            "{tag}: {} players — bespoke {} (head {} / portrait {}), pool {} (floor {:?}, head {}); flag×file {} / {} / {} / {}; shipped {:?} portraits ({:?} unused), {:?} heads ({:?} unused); missing heads {:?}, missing portraits {:?}, flagged without file {:?}",
            s.players, s.bespoke, s.bespoke_head_present, s.bespoke_portrait_present, s.pool_players, s.pool_floor, s.pool_head_present, s.flag1_file1, s.flag1_file0, s.flag0_file1, s.flag0_file0, s.shipped_portraits, s.unused_portraits, s.shipped_heads, s.unused_heads, s.bespoke_missing_head, s.bespoke_missing_portrait, s.flagged_without_file
        );
        assert_eq!(s.players, t.players, "{tag}: players");
        assert_eq!(s.bespoke, t.bespoke, "{tag}: bespoke (head id = art id)");
        assert_eq!(s.bespoke_head_present, t.bespoke_head, "{tag}: bespoke head files");
        assert_eq!(s.bespoke_portrait_present, t.bespoke_portrait, "{tag}: bespoke portrait files");
        // R53's "players_other" is every non-bespoke row; Studio splits that into `pool` (in the
        // measured range) and `other` (head id 0 or below the floor — one row on NHL 09).
        assert_eq!(s.players - s.bespoke, t.pool, "{tag}: non-bespoke population (R53 players_other)");
        assert_eq!(s.pool_head_present, s.pool_players, "{tag}: every pool head ships (R53: 'all')");
        assert_eq!((s.flag1_file1, s.flag1_file0, s.flag0_file1, s.flag0_file0), (t.flag1_file1, t.flag1_file0, t.flag0_file1, t.flag0_file0), "{tag}: hasportrait × file");
        assert_eq!(s.unused_portraits, Some(t.unused_portraits), "{tag}: unused shipped portraits");
        assert_eq!(s.unused_heads, Some(t.unused_heads), "{tag}: unused shipped heads");
        if t.game == "nhl09" {
            for name in ["Nedved", "Andersson", "Marha"] {
                assert!(s.bespoke_missing_head.iter().any(|n| n.contains(name)), "{tag}: {name} is a bespoke player shipped without a head (R53 residuals): {:?}", s.bespoke_missing_head);
            }
            assert!(s.bespoke_missing_portrait.iter().any(|n| n.contains("Zherdev")), "{tag}: {:?}", s.bespoke_missing_portrait);
        }
        if t.game == "nhllegacy" {
            assert!(s.flagged_without_file.iter().any(|n| n.contains("Nagy")), "{tag}: R53 names Nagy among the 2 flagged without a file: {:?}", s.flagged_without_file);
        }
        // Every player with a note has a real disagreement behind it.
        for p in &v.players {
            if let Some(n) = &p.note {
                assert!(n.contains("shipped without asset") || n.contains("hasportrait") || n.contains("flag is clear"), "{tag}: {n}");
            }
        }
    }
}
