//! Receipt 67 — Legacy Recompiled read routes through the public Registry (research 2026-09-28 handoffs 12 §2, 16, 17), on the
//! registered install (read only) against the X360 NHL Legacy loose tree, both over copies of the X360 Legacy anchor database.
//!
//! Containment: attached as the install folder and as its `game` folder, every player's portrait / head status equals the X360
//! Legacy loose tree's, and the card says the check was made against the archive members.
use std::path::{Path, PathBuf};

use studio_api::dto::AssetRootsArgsDto;
use studio_api::{parse_platform, Registry};

pub const RECOMP: &str = r"H:\Emulators\games\XBOX\NHL Legacy Decomp Test";
pub const X360_LEGACY: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn roots(dir: &str, platform: &str) -> AssetRootsArgsDto {
    AssetRootsArgsDto { install_dir: dir.into(), platform: Some(platform.into()), rendering_override: None, fe_override: None }
}
/// Open the (read-only, never saved) X360 Legacy anchor as `game` / `platform`. Nothing here edits it.
fn open(reg: &Registry, game: &str, platform: &str) -> u32 {
    let a = repo("anchors/xbox360/nhllegacy/databases");
    reg.open_workspace_from_paths(Some(game), parse_platform(platform).unwrap(), &s(&a.join("nhlng.db")), None, &s(&a.join("nhlng-meta.xml")), &s(&repo("defs"))).unwrap().project_id
}

#[test]
#[ignore = "the registered Recompiled install under H:/Emulators and the X360 NHL Legacy tree under X:/Games (read only): select with --ignored"]
fn recompiled_containment_reads_the_archive_members_like_the_x360_loose_tree() {
    assert!(Path::new(RECOMP).join("game").is_dir() && Path::new(X360_LEGACY).join("rendering").is_dir(), "trees missing (fails, never skips)");
    let reg = Registry::default();
    let x360 = open(&reg, "nhllegacy", "xbox");
    let loose = reg.player_containment(x360, &roots(X360_LEGACY, "xbox")).unwrap();
    let rc = open(&reg, "legacyrecomp", "recomp");
    for dir in [RECOMP.to_string(), format!(r"{RECOMP}\game")] {
        let v = reg.player_containment(rc, &roots(&dir, "recomp")).unwrap();
        assert_eq!(v.players.len(), loose.players.len());
        // Present on the X360 loose tree ⇒ present here. Missing there ⇒ unknown here, because two archives of this install
        // cannot be listed (`gamedata/gps.big`, a BIG4 without a `.bh`; `gamedata/anim/faceposer.big`, an EB variant the lister
        // does not read), so an id without a member is not proven absent.
        let expect = |x: &str| if x == "missing" { "unknown".to_string() } else { x.to_string() };
        let differ: Vec<_> = v.players.iter().zip(&loose.players).filter(|(a, b)| (a.portrait.clone(), a.head.clone()) != (expect(&b.portrait), expect(&b.head))).map(|(a, b)| (a.full_name.clone(), a.portrait.clone(), b.portrait.clone(), a.head.clone(), b.head.clone())).take(5).collect();
        assert!(differ.is_empty(), "{dir}: statuses differ from the X360 loose tree: {differ:?}; notes {:?}", v.summary.notes);
        let (s, l) = (&v.summary, &loose.summary);
        assert_eq!((s.bespoke, s.bespoke_head_present, s.bespoke_portrait_present, s.pool_head_present), (l.bespoke, l.bespoke_head_present, l.bespoke_portrait_present, l.pool_head_present), "{dir}");
        assert!(s.notes.iter().any(|n| n.contains("checked against this install's own archive members")), "{dir}: {:?}", s.notes);
        assert!(s.notes.iter().any(|n| n.contains("2 archive(s) could not be listed") && n.contains("gps.big") && n.contains("not proven absent")), "{dir}: {:?}", s.notes);
        assert_eq!((s.flag1_file0, s.unused_portraits, s.unused_heads), (0, None, None), "{dir}: nothing claimed absent or unreferenced");
        let unknown = v.players.iter().filter(|p| p.portrait == "unknown").count();
        eprintln!(
            "{dir}: {} players; bespoke {} (head {} / portrait {}), pool heads {}; shipped portraits {:?} heads {:?} (X360 loose {:?} / {:?}); {unknown} portraits unknown (missing on X360)",
            s.players, s.bespoke, s.bespoke_head_present, s.bespoke_portrait_present, s.pool_head_present, s.shipped_portraits, s.shipped_heads, l.shipped_portraits, l.shipped_heads
        );
    }
    // A Recompiled project with a folder holding no archives: unknown, never missing.
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let empty = std::env::temp_dir().join(format!("nhlms-r67-empty-{}-{nanos}", std::process::id()));
    std::fs::create_dir(&empty).unwrap();
    let v = reg.player_containment(rc, &roots(&s(&empty), "recomp"));
    let _ = std::fs::remove_dir(&empty);
    assert!(v.unwrap().players.iter().all(|p| p.portrait == "unknown" && p.head == "unknown"));
}

/// Handoff 16: the speech catalogue, contexts and name calls read the Recompiled archive members (install folder and `game`
/// folder alike) and equal the X360 Legacy loose route — the same unwrapped source hashes and counts; a pbp preview decodes to
/// the same PCM. Negative: a wrong title, a root with neither, and a corrupted chunklzx member (disposable archive copy) refuse.
#[test]
#[ignore = "the registered Recompiled install under H:/Emulators and the X360 NHL Legacy tree under X:/Games (read only; one small archive copied): select with --ignored"]
fn recompiled_speech_routes_read_the_archive_members() {
    let reg = Registry::default();
    let x360 = open(&reg, "nhllegacy", "xbox");
    let rc = open(&reg, "legacyrecomp", "recomp");
    let loose = reg.speech_catalogue(x360, X360_LEGACY, "", false, 5).unwrap();
    let loose_ctx = reg.context_catalogue(x360, X360_LEGACY, "", 5).unwrap();
    let (team, loose_team) = reg.list_teams(x360).unwrap().into_iter().find_map(|t| reg.team_name_calls(x360, t.team_id, X360_LEGACY).ok().filter(|c| c.sample_ids_distinct > 0).map(|c| (t.team_id, c))).expect("a team with recorded name calls");
    let (player, loose_player) = reg.list_players(x360).unwrap().into_iter().take(400).find_map(|p| reg.player_name_calls(x360, p.player_id, X360_LEGACY).ok().filter(|c| c.announcer_sample_ids_distinct > 0 && !c.pbp_names.is_empty()).map(|c| (p.player_id, c))).expect("a player with recorded name calls");
    let hashes = |c: &studio_api::dto::SpeechCatalogueDto| c.sources.iter().map(|s| (s.role.clone(), s.sha256.clone(), s.len)).collect::<Vec<_>>();
    // A previewable pbp sample on the loose route (the first few bank samples are tried).
    let bank = studio_domain::gen7_speech_preview::bank_entries("nhllegacy", studio_domain::Platform::Xbox, Path::new(X360_LEGACY), "pbp_eng").unwrap().1;
    let (sample, loose_wav) = bank.samples.iter().take(40).find_map(|s| reg.gen7_speech_preview(x360, X360_LEGACY, "pbp_eng", s.id).ok().map(|p| (s.id, p.wav_data_url))).expect("a previewable X360 Legacy pbp sample");
    for dir in [RECOMP.to_string(), format!(r"{RECOMP}\game")] {
        let c = reg.speech_catalogue(rc, &dir, "", false, 5).unwrap();
        assert_eq!(hashes(&c), hashes(&loose), "{dir}: same unwrapped sources");
        assert_eq!((c.stats.events, c.stats.sentences, c.stats.bank_samples, c.stats.keywords), (loose.stats.events, loose.stats.sentences, loose.stats.bank_samples, loose.stats.keywords));
        assert!(c.sources.iter().any(|s| s.path.contains("audioboot.big:audio/speech/pbp_eng_samples.sbr (chunklzx")), "{:?}", c.sources.iter().map(|s| &s.path).collect::<Vec<_>>());
        assert!(c.sources.iter().any(|s| s.path.contains("audio/speech/pbp_eng_speecheventdef.csi (stored")));
        assert!(c.notes.iter().any(|n| n.contains("own archive members") && n.contains("not observed")), "{:?}", c.notes);
        let x = reg.context_catalogue(rc, &dir, "", 5).unwrap();
        assert_eq!((x.records, x.triggers, x.speech_items, x.context_items), (loose_ctx.records, loose_ctx.triggers, loose_ctx.speech_items, loose_ctx.context_items), "{dir}: contexts");
        let t = reg.team_name_calls(rc, team, &dir).unwrap();
        assert_eq!((t.sample_ids_distinct, t.bank_resolved_distinct, t.keyword_mentions), (loose_team.sample_ids_distinct, loose_team.bank_resolved_distinct, loose_team.keyword_mentions), "{dir}: team name calls");
        let pc = reg.player_name_calls(rc, player, &dir).unwrap();
        assert_eq!((pc.pbp_names.len(), pc.announcer_sample_ids_distinct, pc.announcer_bank_resolved_distinct), (loose_player.pbp_names.len(), loose_player.announcer_sample_ids_distinct, loose_player.announcer_bank_resolved_distinct), "{dir}: player name calls");
        let p = reg.gen7_speech_preview(rc, &dir, "pbp_eng", sample).unwrap();
        assert_eq!(p.wav_data_url, loose_wav, "{dir}: sample {sample} decodes to the same PCM");
        assert!(p.bank_path.contains("audioboot.big:audio/speech/pbp_eng_samples.sbr") && p.bank_path.contains("chunklzx"), "{}", p.bank_path);
        eprintln!("{dir}: {} events, {} sentences, {} bank samples, {} contexts; team {team} {} samples; player {player} announcer {} samples; sample {sample} PCM equal; bank {}", c.stats.events, c.stats.sentences, c.stats.bank_samples, x.records, t.sample_ids_distinct, pc.announcer_sample_ids_distinct, p.bank_path);
    }
    // Wrong title on the Recompiled root: refused (no loose tree; the archive route is Recompiled only).
    let wrong = open(&reg, "nhl09", "recomp");
    assert!(reg.speech_catalogue(wrong, RECOMP, "", false, 5).is_err());
    // A root with neither a loose tree nor archives.
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let scratch = std::env::temp_dir().join(format!("nhlms-r67-speech-{}-{nanos}", std::process::id()));
    std::fs::create_dir(&scratch).unwrap();
    struct Guard(std::path::PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _g = Guard(scratch.clone());
    let e = reg.speech_catalogue(rc, &s(&scratch), "", false, 5).err().expect("refused").to_string();
    assert!(e.contains("no loose audio/speech directory, and no archives"), "{e}");
    // A disposable copy of audioboot.big whose chunklzx `pbp_eng_samples.sbr` has a corrupted body: refused, never read as stored.
    let game = scratch.join("game");
    std::fs::create_dir(&game).unwrap();
    let mut bytes = std::fs::read(Path::new(RECOMP).join("game").join("audioboot.big")).unwrap();
    let entry = studio_domain::ps4_vault::eb_index(&Path::new(RECOMP).join("game").join("audioboot.big")).unwrap().into_iter().find(|e| e.path.eq_ignore_ascii_case("audio/speech/pbp_eng_samples.sbr")).unwrap();
    let o = entry.offset as usize;
    assert!(tdb_archive_big::eb::is_chunklzx(&bytes[o..o + entry.size as usize]));
    for b in &mut bytes[o + 8..o + 48] {
        *b = 0xFF;
    }
    std::fs::write(game.join("audioboot.big"), &bytes).unwrap();
    let e = reg.speech_catalogue(rc, &s(&scratch), "", false, 5).err().expect("refused").to_string();
    assert!(e.contains("chunklzx member does not decode") && e.contains("not read as stored"), "{e}");
    let e = reg.gen7_speech_preview(rc, &s(&scratch), "pbp_eng", sample).err().expect("refused").to_string();
    assert!(e.contains("chunklzx member does not decode"), "{e}");
}