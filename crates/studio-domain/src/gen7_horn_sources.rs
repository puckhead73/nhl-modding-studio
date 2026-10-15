//! Teams 2.7 (gen7) — **goal / period horn and chant bank sources** for the arena row's audio ids
//! (overnight 2026-09-24, package P4.4; research R78 as corrected by R79, and R81).
//!
//! What is established (R78 §2, exact file presence on ten loose trees NHL 10–Legacy, both consoles):
//! `goalhorn` → `audio/sfx/<id:03>_goal.abk`, `periodhorn` → `audio/sfx/<id:03>_period.abk` for every
//! NHL-club id; the 11–14 non-NHL ids per title ship no loose bank. `chantaudioid` is **not**
//! resolved to a bank (research R97 §1.4, 25 Sep 2026, replacing the R78 "candidate" join): chant
//! banks are loader assets listed by number, twelve of them (055–066) club chants named inside the
//! bank and in the crowd module / control-bank name tables; of the audio-named database fields only
//! the Finnish arena's `chantaudioid` equals a club-bank number (61), and its horn ids are 61 too; no
//! consumer that selects a chant bank has been traced. The value is listed with the same-numbered
//! shipped bank named as a number match only; no link, no edit route.
//! What has **no established binding** (R79): `cityaudioid` / `arenaaudioid` — numerical
//! coincidences with bank ids were reported, not bindings.
//!
//! For an id with no loose file, R81 inspected `nocache.big` (numbered members identical to the loose
//! set on 11 / 11 trees; NHL 09 X360 archive-only), `audioboot.big`, `data0.big` (no numbered horn
//! members), `horns.abk` (S10A table, count 0) and `aemshorn.csi` (a segment-built horn interface).
//! The residual text therefore names those sources and never says "no horn"; two candidates —
//! the `aemshorn` segment-built horn and the shipped-but-unreferenced bank 0 — are named as
//! unestablished. PS3 12 / 13 (and PS3 09 / 10) keep audio in archives: since 26 Sep the probe reads
//! their members too (R98 §1 listed them: `nocache.big` on 12 / 13, `data0.big` `audio/sfx/ram` on 09,
//! `audio.big` on 10), naming each member by its real path.
//!
//! Research R85 (2026-09-24 handoff 03 §1) adds the **loader asset table** `audio/na_en_loadfile.xml`
//! (loose on X360 09–Legacy and PS3 11 / 14 / 15 / Legacy; an `audioboot.big` member on every tree,
//! RefPack-wrapped on X360 09): `AemsSfxGoalHornBank` / `AemsSfxPeriodHornBank` list ids 0–29, 100
//! (+ 194 from NHL 14), `AemsCrowdTeamChantBank` 73 three-digit ids, `AemsCrowdGoalieChantBank` 45
//! four-digit ids keyed by **goalie announcer audio ids** (not `chantaudioid`); every listed bank is
//! shipped and every shipped bank is listed on 16 / 16 trees, and no residual id is listed — "an
//! unlisted id has no bank to load". What the engine then does stays runtime (card H-HORN-1).
//!
//! This module **probes the open install** (loose `audio/sfx`, the numbered members of the audio
//! archives, and the loader table) and states what it found; it does not decode any bank.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The numbered banks found on this install, per family.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7HornSources {
    pub game_id: String,
    pub platform: String,
    pub sfx_dir: String,
    pub sfx_dir_present: bool,
    pub loose_goal: BTreeSet<u32>,
    pub loose_period: BTreeSet<u32>,
    pub loose_chant: BTreeSet<u32>,
    /// Numbered members (`audio/sfx/…/<id>_goal.abk` …) of the audio archives beside the root that
    /// list: `nocache.big` (EB), `data0.big` and `audio.big` (BIG4 + `.bh`, NHL 09 / 10). Ids map to
    /// `archive:member path` (the first archive that carries the id).
    pub archive_goal: std::collections::BTreeMap<u32, String>,
    pub archive_period: std::collections::BTreeMap<u32, String>,
    pub archive_chant: std::collections::BTreeMap<u32, String>,
    /// One status line per archive inspected.
    pub archive_status: Vec<String>,
    pub horns_abk_len: Option<u64>,
    pub aemshorn_csi_present: bool,
    /// Why the probe was not performed (PS3 12 / 13 archive-only audio; a title outside the gen7 loose-tree set).
    pub refusal: Option<String>,
    /// The loader asset table (R85), when read.
    pub loader: Option<LoaderTable>,
}

/// `audio/na_en_loadfile.xml`: the horn / chant asset ids the loader binds to bank files.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoaderTable {
    /// Where it was read: the loose path, or `<archive>:<member> (<wrapper>)`.
    pub source: String,
    pub goal: BTreeSet<u32>,
    pub period: BTreeSet<u32>,
    /// `AemsCrowdTeamChantBank` (three-digit stems).
    pub team_chant: BTreeSet<u32>,
    /// `AemsCrowdGoalieChantBank` (four-digit stems; keyed by goalie announcer audio id, R85 §2).
    pub goalie_chant: BTreeSet<u32>,
    pub horns_bank_listed: bool,
    pub horns_event_definition_listed: bool,
}

impl LoaderTable {
    /// Parse the manifest's `<Asset assettype="…" path="…" descr="…"/>` rows (a flat attribute
    /// grammar; the file is plain ASCII XML on every measured tree).
    pub fn parse(source: &str, xml: &str) -> LoaderTable {
        let mut t = LoaderTable { source: source.to_string(), ..Default::default() };
        for line in xml.split('<') {
            let Some(rest) = line.strip_prefix("Asset ") else { continue };
            let attr = |name: &str| -> Option<&str> {
                let key = format!("{name}=\"");
                let i = rest.find(&key)? + key.len();
                let j = rest[i..].find('"')? + i;
                Some(&rest[i..j])
            };
            let (Some(ty), Some(path)) = (attr("assettype"), attr("path")) else { continue };
            let file = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
            match ty {
                "AemsSfxGoalHornBank" => {
                    if let Some(id) = stem_id(&file, "_goal.abk") { t.goal.insert(id); }
                }
                "AemsSfxPeriodHornBank" => {
                    if let Some(id) = stem_id(&file, "_period.abk") { t.period.insert(id); }
                }
                "AemsCrowdTeamChantBank" => {
                    if let Some(id) = stem_id(&file, "_chant.abk") { t.team_chant.insert(id); }
                }
                "AemsCrowdGoalieChantBank" => {
                    if let Some(id) = stem_id(&file, "_chant.abk") { t.goalie_chant.insert(id); }
                }
                "AemsHornsBank" => t.horns_bank_listed = true,
                "AemsHornsEventDefinition" => t.horns_event_definition_listed = true,
                _ => {}
            }
        }
        t
    }
}

/// Read a member of an EB or BIG4 archive and unwrap RefPack / chunkzip.
fn archive_member_bytes(archive: &Path, member_path: &str) -> Option<(Vec<u8>, &'static str)> {
    use std::io::{Read, Seek, SeekFrom};
    let (offset, size) = crate::gen7_speech_preview::locate_member(archive, member_path)?;
    let mut f = std::fs::File::open(archive).ok()?;
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut raw = vec![0u8; usize::try_from(size).ok()?];
    f.read_exact(&mut raw).ok()?;
    if raw.starts_with(tdb_archive_codec::MAGIC_CHUNKZIP) {
        return Some((tdb_archive_codec::decompress_chunkzip(&raw).ok()?, "chunkzip"));
    }
    if raw.starts_with(tdb_archive_codec::MAGIC_CHUNKLZX) {
        return Some((tdb_archive_codec::decompress_chunklzx(&raw).ok()?, "chunklzx"));
    }
    if tdb_archive_codec::is_refpack(&raw) {
        return Some((tdb_archive_codec::refpack_decompress(&raw).ok()?, "refpack"));
    }
    Some((raw, "stored"))
}

impl LoaderTable {
    /// The loader table: loose `audio/na_en_loadfile.xml`, else the `audioboot.big` member
    /// (RefPack on 09 / 10, chunkzip on PS3 11–15, chunklzx on X360 14–Legacy per the R85 copies).
    pub fn read(install_root: &Path) -> Option<LoaderTable> {
    let loose = install_root.join("audio").join("na_en_loadfile.xml");
    if let Ok(b) = std::fs::read(&loose) {
        return Some(LoaderTable::parse(&loose.to_string_lossy(), &String::from_utf8_lossy(&b)));
    }
    for name in ["audioboot.big", "data0.big", "audio.big", "nocache.big"] {
        let archive = install_root.join(name);
        if !archive.is_file() {
            continue;
        }
        if let Some((b, wrapper)) = archive_member_bytes(&archive, "audio/na_en_loadfile.xml") {
            return Some(LoaderTable::parse(&format!("{name}:audio/na_en_loadfile.xml ({wrapper})"), &String::from_utf8_lossy(&b)));
        }
    }
    None
    }
}

fn stem_id(name: &str, suffix: &str) -> Option<u32> {
    let stem = name.strip_suffix(suffix)?;
    if stem.is_empty() || stem.len() > 4 || !stem.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    stem.parse().ok()
}

impl Gen7HornSources {
    /// Probe the install. Read-only: directory listing + the archive's file table.
    pub fn probe(game_id: &str, platform: crate::Platform, install_root: &Path) -> Gen7HornSources {
        let plat = match platform {
            crate::Platform::Xbox | crate::Platform::Recomp => "xbox",
            crate::Platform::Ps3 => "ps3",
            crate::Platform::Ps4 => "ps4",
        };
        let mut out = Gen7HornSources { game_id: game_id.to_string(), platform: plat.to_string(), ..Default::default() };
        if plat == "ps4" {
            out.refusal = Some("PS4 titles select horns through the `goalhorns.csv` catalogue (R50), not numbered `.abk` banks".into());
            return out;
        }
        if matches!(game_id, "nhl07" | "nhl08") {
            out.refusal = Some("NHL 07 / 08 predate the numbered `audio/sfx` horn banks (research R78 covers NHL 10–Legacy; NHL 09 X360 archive members)".into());
            return out;
        }
        let sfx: PathBuf = match crate::assets::resolve_install_path(install_root, "audio/sfx") {
            Some(p) => p,
            None => {
                out.refusal = Some("audio/sfx escapes the install root".into());
                return out;
            }
        };
        out.sfx_dir = sfx.to_string_lossy().into_owned();
        out.sfx_dir_present = sfx.is_dir();
        if out.sfx_dir_present {
            if let Ok(rd) = std::fs::read_dir(&sfx) {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().to_ascii_lowercase();
                    if let Some(id) = stem_id(&name, "_goal.abk") {
                        out.loose_goal.insert(id);
                    } else if let Some(id) = stem_id(&name, "_period.abk") {
                        out.loose_period.insert(id);
                    } else if let Some(id) = stem_id(&name, "_chant.abk") {
                        out.loose_chant.insert(id);
                    }
                }
            }
            out.horns_abk_len = std::fs::metadata(sfx.join("horns.abk")).ok().map(|m| m.len());
            out.aemshorn_csi_present = sfx.join("aemshorn.csi").is_file();
        }
        // Archives beside the root (research R81 §1, R98 §1): `nocache.big` (EB), `data0.big` and
        // `audio.big` (BIG4 with its `.bh` sidecar; NHL 09 keeps the numbered banks in `data0.big`
        // `audio/sfx/ram`, PS3 NHL 10 in `audio.big`).
        for archive_name in ["nocache.big", "data0.big", "audio.big"] {
            let archive = install_root.join(archive_name);
            if !archive.is_file() {
                out.archive_status.push(format!("{archive_name} not present beside this root"));
                continue;
            }
            let magic = std::fs::File::open(&archive).and_then(|mut f| {
                use std::io::Read;
                let mut m = [0u8; 4];
                f.read_exact(&mut m)?;
                Ok(m)
            });
            let members: Result<Vec<String>, String> = match magic {
                Ok(m) if &m == b"EB\0\x03" => crate::kit_files::list_eb_members(&archive),
                Ok(m) if &m == b"BIG4" => {
                    let bh = archive.with_extension("bh");
                    match (std::fs::File::open(&archive), std::fs::read(&bh)) {
                        // Directory only: bind would hash the whole archive on every arena view.
                        (Ok(mut f), Ok(sidecar)) => tdb_archive_big::big4::BoundBig4::read_directory(&mut f, &sidecar)
                            .map(|entries| entries.iter().map(|e| e.path.clone()).collect())
                            .map_err(|e| e.to_string()),
                        (_, Err(e)) => Err(format!("{}: {e}", bh.display())),
                        (Err(e), _) => Err(e.to_string()),
                    }
                }
                Ok(m) => Err(format!("unrecognised archive magic {m:?}")),
                Err(e) => Err(e.to_string()),
            };
            match members {
                Ok(members) => {
                    let (mut g, mut p, mut c) = (0, 0, 0);
                    for m in members {
                        let lower = m.replace('\\', "/").to_ascii_lowercase();
                        if !lower.starts_with("audio/sfx/") {
                            continue;
                        }
                        let name = lower.rsplit('/').next().unwrap_or(&lower).to_string();
                        let at = format!("{archive_name}:{}", m.replace('\\', "/"));
                        if let Some(id) = stem_id(&name, "_goal.abk") {
                            out.archive_goal.entry(id).or_insert_with(|| at.clone());
                            g += 1;
                        } else if let Some(id) = stem_id(&name, "_period.abk") {
                            out.archive_period.entry(id).or_insert_with(|| at.clone());
                            p += 1;
                        } else if let Some(id) = stem_id(&name, "_chant.abk") {
                            out.archive_chant.entry(id).or_insert_with(|| at.clone());
                            c += 1;
                        }
                    }
                    out.archive_status.push(format!("{archive_name} listed: {g} goal, {p} period, {c} chant numbered members"));
                }
                Err(e) => out.archive_status.push(format!("{archive_name} present but not listed: {e}")),
            }
        }
        out.loader = LoaderTable::read(install_root);
        out
    }

    fn inspected(&self) -> String {
        let mut s = String::from("inspected: loose `audio/sfx`");
        for a in self.archive_status.iter().filter(|a| a.contains(" listed: ")) {
            s.push_str(&format!(", `{}` members", a.split(' ').next().unwrap_or("archive")));
        }
        match self.horns_abk_len {
            Some(_) => s.push_str("; `horns.abk` carries no samples (S10A count 0, research R81)"),
            None => s.push_str("; no `horns.abk`"),
        }
        match &self.loader {
            Some(l) => s.push_str(&format!("; the loader asset table `{}` lists {} goal / {} period / {} team-chant / {} goalie-chant banks", l.source, l.goal.len(), l.period.len(), l.team_chant.len(), l.goalie_chant.len())),
            None => s.push_str("; loader asset table `audio/na_en_loadfile.xml` not found loose or in the audio archives"),
        }
        s
    }

    /// The sound-bank file a goal / period horn value opens in the bank browser (26 Sep 2026):
    /// only when the numbered bank was found (loose, or an archive member) **and** the bank
    /// listing covers this title ([`crate::gen7_sfx_banks::coverage`]). Chant and audio-id fields
    /// never link — no selecting consumer is traced (R97 §1.4 / R79).
    pub fn bank_file(&self, field: &str, value: i64) -> Option<String> {
        if self.refusal.is_some() || value <= 0 {
            return None;
        }
        let id = u32::try_from(value).ok()?;
        let (family, loose, archive) = match field {
            "goalhorn" => ("goal", &self.loose_goal, &self.archive_goal),
            "periodhorn" => ("period", &self.loose_period, &self.archive_period),
            _ => return None,
        };
        if !loose.contains(&id) && !archive.contains_key(&id) {
            return None;
        }
        let platform = match self.platform.as_str() {
            "ps3" => crate::Platform::Ps3,
            "xbox" => crate::Platform::Xbox,
            _ => return None,
        };
        crate::gen7_sfx_banks::coverage(&self.game_id, platform).ok()?;
        Some(format!("{id:03}_{family}.abk"))
    }

    /// R85 §1: is `id` listed for `family` (`goal` / `period` / `chant`) in the loader table?
    fn loader_lists(&self, family: &str, id: u32) -> Option<bool> {
        let l = self.loader.as_ref()?;
        Some(match family {
            "goal" => l.goal.contains(&id),
            "period" => l.period.contains(&id),
            _ => l.team_chant.contains(&id) || l.goalie_chant.contains(&id),
        })
    }

    /// The source reading for one arena-row audio field: `(resolved text, note)`; `None` when the
    /// field is not one of the horn / chant / audio-id columns or the probe was refused.
    pub fn resolve(&self, field: &str, value: i64) -> Option<(Option<String>, String)> {
        if let Some(r) = &self.refusal {
            return matches!(field, "goalhorn" | "periodhorn" | "chantaudioid" | "cityaudioid" | "arenaaudioid").then(|| (None, r.clone()));
        }
        let id = u32::try_from(value).ok();
        let file = |family: &str, id: u32| format!("audio/sfx/{id:03}_{family}.abk");
        match field {
            "goalhorn" | "periodhorn" => {
                let (family, loose, archive) = if field == "goalhorn" { ("goal", &self.loose_goal, &self.archive_goal) } else { ("period", &self.loose_period, &self.archive_period) };
                let Some(id) = id else { return Some((None, format!("value {value} is not a bank id"))) };
                if value == 0 {
                    return Some((None, format!("0: no {family} horn selected; bank 0 is shipped but unreferenced (a candidate default, unestablished — research R81)")));
                }
                let listed = match self.loader_lists(family, id) {
                    Some(true) => "; listed in the loader asset table (R85)",
                    Some(false) => "; NOT listed in the loader asset table although the bank is shipped (R85 measured none such — report it)",
                    None => "",
                };
                if loose.contains(&id) {
                    return Some((Some(file(family, id)), format!("{family} horn bank by exact file presence (research R78 §3, NHL clubs ids 1–29; loose `audio/sfx`){listed}")));
                }
                if let Some(at) = archive.get(&id) {
                    let arc = at.split(':').next().unwrap_or(at);
                    return Some((Some(at.clone()), format!("{family} horn bank as a `{arc}` member (by name — research R81 §3 / R98 §1; the sound-bank browser opens it where the title is listed){listed}")));
                }
                let loader_note = match self.loader_lists(family, id) {
                    Some(false) => " and no loader asset for this id (R85: the loader binds ids to banks through `audio/na_en_loadfile.xml`; an unlisted id has no bank to load)",
                    Some(true) => " although the loader asset table lists it (R85 measured no such case — report it)",
                    None => "",
                };
                Some((
                    None,
                    format!(
                        "no {family} horn bank found in the loose folder or the audio archives ({}; research R81: `audioboot.big` / `data0.big` carry no numbered horn members){loader_note}. Not \"no horn\": two unestablished candidates — the segment-built horn of the `aemshorn` module{} and the unreferenced bank 0 (card H-HORN-1)",
                        self.inspected(),
                        if self.aemshorn_csi_present { " (present)" } else { "" }
                    ),
                ))
            }
            "chantaudioid" => {
                let Some(id) = id else { return Some((None, format!("value {value} is not a bank id"))) };
                // R97 §1.4 (25 Sep 2026): measured = numbered loader assets and the club names
                // inside twelve banks; untraced = what selects a chant bank. Never resolved, never linked.
                const UNTRACED: &str = "Chant banks are loader assets listed by number; twelve (055–066) are club chants named inside the bank and in the crowd module / control-bank name tables. Of the audio-named database fields (17–20 per title) only the Finnish arena's `chantaudioid` equals a club-bank number (61), and that arena's goal-horn and period-horn ids are 61 as well; no club's own arena or team row carries its bank number. No consumer that selects a chant bank has been traced (research R97 §1.4). Listing only; no edit route";
                if value == 0 {
                    return Some((None, format!("0: no chant bank carries this number. {UNTRACED}")));
                }
                let bank = format!("{id:03}_chant.abk");
                let present = self.loose_chant.contains(&id) || self.archive_chant.contains_key(&id);
                let where_ = if self.loose_chant.contains(&id) { "loose".to_string() } else { self.archive_chant.get(&id).cloned().unwrap_or_default() };
                let goalie = self.loader.as_ref().is_some_and(|l| l.goalie_chant.contains(&id));
                let listed = match self.loader_lists("chant", id) {
                    Some(true) => if goalie { "; the loader lists it as a **goalie** chant bank (keyed by a goalie's announcer audio id, R85 §2)" } else { "; the loader lists it as a team chant bank (R85)" },
                    Some(false) if present => "; shipped but NOT listed in the loader asset table (R85 §1 assumed every shipped chant bank is listed; the developer's probe found this one unlisted)",
                    Some(false) => "; not listed in the loader asset table",
                    None => "",
                };
                let club = if (55..=66).contains(&id) { " — one of the twelve club-chant banks" } else { "" };
                if present {
                    Some((None, format!("a chant bank with the same number is shipped: `{bank}` ({where_}){club}{listed}. A number match, not a binding. {UNTRACED}")))
                } else {
                    let unshipped = if self.loader_lists("chant", id) == Some(true) { " — listed by the loader but no such file is shipped (R85 §1 assumed none such; the developer's probe found this one — report it)" } else { "" };
                    Some((None, format!("no chant bank `{bank}` is shipped for this number{listed}{unshipped}. {UNTRACED}")))
                }
            }
            "cityaudioid" | "arenaaudioid" => Some((None, format!("{field}: no established binding to a numbered `.abk` bank (research R79 — the numerical hits with chant / goal ids are coincidences of small id spaces; the searched scope was the loose `audio/sfx` banks); speech sample-group id per handoff 07, tag open (Q-R35-1)"))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_a_synthetic_loose_tree_and_reads_every_column_class_by_the_research_rules() {
        let tmp = std::env::temp_dir().join(format!("nhlms-horns-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let sfx = tmp.join("audio/sfx");
        std::fs::create_dir_all(&sfx).unwrap();
        for f in ["000_goal.abk", "001_goal.abk", "029_goal.abk", "100_goal.abk", "001_period.abk", "1011_chant.abk", "012_chant.abk", "horns.abk", "aemshorn.csi", "notes.txt"] {
            std::fs::write(sfx.join(f), b"x").unwrap();
        }
        std::fs::write(
            tmp.join("audio/na_en_loadfile.xml"),
            "<AssetManifest>\n  <AssetType name=\"AemsSfxGoalHornBank\" filetype=\"Memory\" loadtype=\"temp\"/>\n  <Asset assettype=\"AemsSfxGoalHornBank\" path=\"audio/sfx/000_goal.abk\" descr=\"0\"/>\n  <Asset assettype=\"AemsSfxGoalHornBank\" path=\"audio/sfx/001_goal.abk\" descr=\"1\"/>\n  <Asset assettype=\"AemsSfxPeriodHornBank\" path=\"audio/sfx/001_period.abk\" descr=\"1\"/>\n  <Asset assettype=\"AemsCrowdTeamChantBank\" path=\"audio/sfx/012_chant.abk\" descr=\"12\"/>\n  <Asset assettype=\"AemsCrowdGoalieChantBank\" path=\"audio/sfx/1011_chant.abk\" descr=\"1011\"/>\n  <Asset assettype=\"AemsHornsBank\" path=\"audio/sfx/horns.abk\" descr=\"horns\"/>\n  <Asset assettype=\"AemsHornsEventDefinition\" path=\"audio/sfx/aemsHorn.csi\" descr=\"aemsHorn\"/>\n</AssetManifest>\n",
        )
        .unwrap();
        let p = Gen7HornSources::probe("nhl15", crate::Platform::Xbox, &tmp);
        assert!(p.refusal.is_none() && p.sfx_dir_present);
        assert_eq!(p.loose_goal.iter().copied().collect::<Vec<_>>(), vec![0, 1, 29, 100]);
        assert_eq!(p.loose_period.iter().copied().collect::<Vec<_>>(), vec![1]);
        assert_eq!(p.loose_chant.iter().copied().collect::<Vec<_>>(), vec![12, 1011]);
        assert_eq!(p.horns_abk_len, Some(1));
        assert!(p.aemshorn_csi_present);
        assert!(p.archive_status.iter().all(|a| a.contains("not present")), "{:?}", p.archive_status);
        let l = p.loader.as_ref().expect("loader table read");
        assert_eq!((l.goal.len(), l.period.len(), l.team_chant.len(), l.goalie_chant.len(), l.horns_bank_listed, l.horns_event_definition_listed), (2, 1, 1, 1, true, true));
        let (r, n) = p.resolve("goalhorn", 1).unwrap();
        assert_eq!(r.as_deref(), Some("audio/sfx/001_goal.abk"));
        assert!(n.contains("exact file presence") && n.contains("listed in the loader asset table"), "{n}");
        let (_, n) = p.resolve("goalhorn", 29).unwrap();
        assert!(n.contains("NOT listed in the loader asset table although the bank is shipped"), "{n}");
        let (r, n) = p.resolve("goalhorn", 41).unwrap();
        assert!(r.is_none() && n.starts_with("no goal horn bank found in the loose folder or the audio archives") && n.contains("no loader asset for this id") && n.contains("aemshorn") && n.contains("bank 0") && n.contains("Not \"no horn\""), "{n}");
        let (r, n) = p.resolve("periodhorn", 1).unwrap();
        assert_eq!(r.as_deref(), Some("audio/sfx/001_period.abk"));
        assert!(n.contains("period"));
        let (r, n) = p.resolve("goalhorn", 0).unwrap();
        assert!(r.is_none() && n.contains("unreferenced"));
        // R97 §1.4: chant values are never resolved or linked; the same-numbered bank is a number match.
        let (r, n) = p.resolve("chantaudioid", 12).unwrap();
        assert!(r.is_none(), "chant ids resolve to nothing");
        assert!(n.contains("`012_chant.abk` (loose)") && n.contains("not a binding") && n.contains("team chant bank") && n.contains("No consumer that selects a chant bank has been traced") && n.contains("Finnish arena") && n.contains("no edit route"), "{n}");
        for bad in ["silen", "fallback", "default chant", "candidate"] {
            assert!(!n.to_lowercase().contains(bad), "no {bad} claim: {n}");
        }
        let (r, n) = p.resolve("chantaudioid", 1011).unwrap();
        assert!(r.is_none() && n.contains("goalie") && n.contains("1011_chant.abk"), "{n}");
        let (r, n) = p.resolve("chantaudioid", 33).unwrap();
        assert!(r.is_none() && n.starts_with("no chant bank `033_chant.abk` is shipped") && n.contains("No consumer"), "{n}");
        assert!(p.resolve("chantaudioid", 0).unwrap().1.contains("No consumer"));
        assert_eq!(p.bank_file("chantaudioid", 12), None, "chants never link");
        assert_eq!(p.bank_file("cityaudioid", 1), None);
        let (r, n) = p.resolve("cityaudioid", 61).unwrap();
        assert!(r.is_none() && n.contains("no established binding"));
        assert!(p.resolve("seat1zone1", 5).is_none());
        // Refusals: PS4, NHL 07 / 08.
        // PS3 12 / 13 are probed since 26 Sep (R98 members); with no archives beside the root the
        // status says so and nothing resolves.
        let r = Gen7HornSources::probe("nhl12", crate::Platform::Ps3, &tmp);
        assert!(r.refusal.is_none() && r.archive_status.iter().any(|s| s.contains("nocache.big not present")), "{:?}", r.archive_status);
        assert!(Gen7HornSources::probe("nhl17", crate::Platform::Ps4, &tmp).refusal.is_some());
        assert!(Gen7HornSources::probe("nhl07", crate::Platform::Xbox, &tmp).refusal.is_some());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
