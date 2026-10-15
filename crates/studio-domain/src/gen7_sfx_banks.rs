//! Gen7 (PS3 / Xbox 360) **sound banks** (`.abk` + their `.ast` stream file) — listing, the named
//! picker tree, stream framing and the **diagnostic** decode of EALayer3 streams / ram records, on
//! the research lane's measured populations only (R82 / R89 / R91 / R92 / R94 / R98 / R100;
//! consumed 25 – 26 Sep 2026).
//!
//! What is offered, per population:
//!
//! | Population | where the banks / streams are | listed | tree | framed | diagnostic decode |
//! |---|---|---|---|---|---|
//! | PS3 NHL 11 / 14 / 15 / Legacy (R89 / R91) | loose `audio/sfx` | yes | yes | yes | EALayer3 v1, channel config 0 / 1 (R92 construction) |
//! | PS3 NHL 09 / 10 / 12 / 13 (R98 / R100) | each title's own archive members (NHL 10: four streams loose) | yes | yes | yes | same route; R100 decoded named vectors only |
//! | Xbox 360 NHL 10 → Legacy, loose (R89 / R91) | loose `audio/sfx` | yes | yes | yes | EA-XMA **through an installed XMA2 helper** on the measured shapes (receipts 30 / 33 / 50 / 53 / 56); otherwise framed only |
//! | Legacy Recompiled (receipt 59; R128 / R138) | its own archive members (`game/nocache.big`, `audioboot*.big`, `cacheaudio.big`) — byte-identical to the X360 Legacy files | yes | yes | yes | the X360 Legacy rules for these bytes, labelled; runtime unobserved |
//! | Xbox 360 NHL 09 | archives + loose copies, packet-header variant (R98b) | refused with the reason | | | |
//! | PS4 | not this container | | | | |
//!
//! Every stream is read only after its file's `S10S` header id is checked against the bank's word
//! 0x2c **on the same handle** (review 2026-09-26 F1). Archive members are read in place from the
//! archive, bounded to the member's stored extent; a wrapped (RefPack / chunkzip) stream member is
//! refused rather than inflated, because R98 found every `.ast` stored.
//!
//! Nothing here writes, claims which sample the game plays, or claims the decoded PCM is what the
//! console renders (`AST_FRAMING.md` §6 "not established"). Since 26 Sep, channel configs 3 / 5
//! preview one stereo sub-stream at a time by the research's **inferred** de-interleave (EA frame
//! i → sub-stream i mod N, R91 / R92; not reference-validated), and stream loops preview as the
//! stub's PCM frame followed by the body (R91 §1: the body is the next stream at the stub's end,
//! stub + body = the record's count), played once. The unknown words `X` / `S` are carried behind
//! disclosure and never rewritten (R94).
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{DomainError, Result};
use tdb_snd::abk::{self, AbkBank, BankPlatform, RecordBody};

/// Where one title keeps its banks and streams (R82 / R89 / R98 §1).
struct Layout {
    population: &'static str,
    /// Directories (relative to the root) that hold `.abk` files / members.
    bank_dirs: &'static [&'static str],
    /// Directories that hold `.ast` files / members.
    stream_dirs: &'static [&'static str],
    /// Archives beside the root carrying bank members (none = loose only).
    bank_archives: &'static [&'static str],
    /// Archives beside the root carrying stream members.
    stream_archives: &'static [&'static str],
    /// R98 / R100 notes on structure versus decoded vectors for this title.
    evidence: &'static str,
}

const LOOSE: &[&str] = &["audio/sfx"];

fn layout(game_id: &str, platform: crate::Platform) -> Result<Layout> {
    let loose = |population| Layout {
        population,
        bank_dirs: LOOSE,
        stream_dirs: LOOSE,
        bank_archives: &[],
        stream_archives: &[],
        evidence: "Structure: R89 / R91 framed every referenced stream of the loose banks. Decode: R92 decoded named vectors; other entries use the same route by class, not individually witnessed.",
    };
    match platform {
        crate::Platform::Ps4 => Err(DomainError::Validation("PS4 titles do not use ABKC banks (their sound lives in SBR / SNS banks and PTCH patches); nothing to list here".into())),
        crate::Platform::Ps3 => match game_id {
            "nhl11" | "nhl14" | "nhl15" | "nhllegacy" => Ok(loose("R89 / R91 (loose PS3 NHL 11 / 14 / 15 / Legacy)")),
            "nhl09" => Ok(Layout {
                population: "R98 / R100 (PS3 NHL 09: banks in audioboot.big / data0.big audio/sfx/ram, streams in data0.big audio/sfx/streams)",
                bank_dirs: &["audio/sfx/ram"],
                stream_dirs: &["audio/sfx/streams"],
                bank_archives: &["audioboot.big", "data0.big"],
                stream_archives: &["data0.big"],
                evidence: "Structure: R98 framed all 506 referenced streams of the 186 bank members (0 refused). Decode: R100 decoded only playercalls 342, sfx 132 and the crowds 0 loop stub; other entries use the same route by class, not individually witnessed.",
            }),
            "nhl10" => Ok(Layout {
                population: "R98 / R100 (PS3 NHL 10: banks in audio.big / audioboot.big / audioboot2.big / stadium.big, streams in audio.big + four loose audio/sfx files)",
                bank_dirs: LOOSE,
                stream_dirs: LOOSE,
                bank_archives: &["audio.big", "audioboot.big", "audioboot2.big", "stadium.big"],
                stream_archives: &["audio.big"],
                evidence: "Structure: R98 framed all 1 108 referenced streams of the 199 bank members (0 refused). Decode: R100 decoded only playercalls 382, sfx 449, crowds 144 and the crowds 121 loop stub; other entries use the same route by class, not individually witnessed.",
            }),
            "nhl12" => Ok(Layout {
                population: "R98 / R100 (PS3 NHL 12: banks in nocache.big / audioboot.big / audioboot2.big, streams in nocache.big)",
                bank_dirs: LOOSE,
                stream_dirs: LOOSE,
                bank_archives: &["nocache.big", "audioboot.big", "audioboot2.big"],
                stream_archives: &["nocache.big"],
                evidence: "Structure: R98 framed all 3 719 referenced streams of the 219 bank members (0 refused). Decode: R100 decoded only playercalls 0, sfx 92, crowds 6 and the crowds_winterclassic 9 loop stub; other entries use the same route by class, not individually witnessed.",
            }),
            "nhl13" => Ok(Layout {
                population: "R98 / R100 (PS3 NHL 13: banks in nocache.big / audioboot.big / audioboot2.big, streams in nocache.big + cacheaudio.big)",
                bank_dirs: LOOSE,
                stream_dirs: LOOSE,
                bank_archives: &["nocache.big", "audioboot.big", "audioboot2.big"],
                stream_archives: &["nocache.big", "cacheaudio.big"],
                evidence: "Structure: R98 framed all 3 708 referenced streams of the 213 bank members (0 refused). Decode: R100 decoded only playercalls 0, sfx 0, crowds 18 and the crowds 113 loop stub; other entries use the same route by class, not individually witnessed.",
            }),
            _ => Err(DomainError::Validation(format!("{game_id} PS3: no measured ABKC population (R89 / R98 cover PS3 NHL 09 → Legacy)"))),
        },
        // Receipt 59 (research 2026-09-28 handoff 04 §2 / §6, R138): the registered Recompiled install keeps its banks as stored
        // members of its own archives under `game\` (no loose `audio`); every member path is the X360 Legacy file's, byte-identical
        // (R128). The attached folder may be the install or its `game\` folder; archives that are absent are skipped.
        crate::Platform::Recomp => match game_id {
            "legacyrecomp" | "nhllegacy-recompiled" | "nhllegacy" => Ok(Layout {
                population: "R138 (Legacy Recompiled: banks in game/nocache.big / audioboot.big / audioboot2.big, streams in game/nocache.big + cacheaudio.big; the X360 Legacy files, byte-identical, R128)",
                bank_dirs: LOOSE,
                stream_dirs: LOOSE,
                bank_archives: &["game/nocache.big", "game/audioboot.big", "game/audioboot2.big", "nocache.big", "audioboot.big", "audioboot2.big"],
                stream_archives: &["game/nocache.big", "game/cacheaudio.big", "nocache.big", "cacheaudio.big"],
                evidence: "Structure and decode: the X360 Legacy measurements (R89 / R91 / R92 / R139 / R140b) apply to these members because their bytes are the X360 Legacy files (R128 / R138). Which copy the recompiled executable loads is not observed.",
            }),
            _ => Err(DomainError::Validation(format!("{game_id} on Legacy Recompiled: only the Legacy title runs there"))),
        },
        crate::Platform::Xbox => match game_id {
            "nhl10" | "nhl11" | "nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy" => Ok(loose("R89 / R91 (loose Xbox 360 NHL 10 → Legacy)")),
            // Receipt 36 (research R98 / R98b / R131): loose `audio/sfx/ram` + `audio/sfx/streams`, the
            // archive members byte-identical copies; the XMA packet-header variant is read by the
            // count rule only (`header_rule`).
            "nhl09" => Ok(Layout {
                population: "R98 / R131 (Xbox 360 NHL 09: banks in audio/sfx/ram, streams in audio/sfx/streams — loose, with byte-identical copies in audioboot.big / data0.big)",
                bank_dirs: &["audio/sfx/ram"],
                stream_dirs: &["audio/sfx/streams"],
                bank_archives: &["audioboot.big", "data0.big"],
                stream_archives: &["data0.big"],
                evidence: "Structure: R98 framed every referenced stream of the 186 banks (0 refused). Decode: R131 swept every record through the XMA route with the count rule — 471 / 471 non-loop mono / stereo stream entries and 1 983 / 1 985 in-bank segments decode to exactly their declared counts; the two short ones refuse. Packet headers are NHL 09's constant variant (R98b) and are not read as frame counts.",
            }),
            _ => Err(DomainError::Validation(format!("{game_id} Xbox 360: no measured ABKC population (R89 covers NHL 10 → Legacy)"))),
        },
    }
}

/// Why a `(platform, game)` is or is not inside the measured bank population.
pub fn coverage(game_id: &str, platform: crate::Platform) -> Result<&'static str> {
    layout(game_id, platform).map(|l| l.population)
}

fn platform_label(p: crate::Platform) -> &'static str {
    match p {
        crate::Platform::Xbox => "xbox",
        crate::Platform::Recomp => "recomp",
        crate::Platform::Ps3 => "ps3",
        crate::Platform::Ps4 => "ps4",
    }
}

/// One copy of a bank or stream file: loose on disk, or an archive member read in place.
#[derive(Debug, Clone)]
enum Loc {
    Loose(PathBuf),
    Member { archive: PathBuf, archive_name: String, member: String, offset: u64, size: u64 },
}

impl Loc {
    fn describe(&self) -> String {
        match self {
            Loc::Loose(p) => p.to_string_lossy().into_owned(),
            Loc::Member { archive_name, member, .. } => format!("{archive_name}:{member}"),
        }
    }
    /// `(file, base, len)` — the stored extent this copy occupies.
    fn extent(&self) -> std::io::Result<(std::fs::File, u64, u64)> {
        match self {
            Loc::Loose(p) => {
                let f = std::fs::File::open(p)?;
                let len = f.metadata()?.len();
                Ok((f, 0, len))
            }
            Loc::Member { archive, offset, size, .. } => Ok((std::fs::File::open(archive)?, *offset, *size)),
        }
    }
    fn read_stored(&self) -> Result<Vec<u8>> {
        let (mut f, base, len) = self.extent().map_err(|e| DomainError::Validation(format!("{}: {e}", self.describe())))?;
        f.seek(SeekFrom::Start(base)).map_err(|e| DomainError::Validation(format!("{}: seek: {e}", self.describe())))?;
        let mut buf = vec![0u8; len as usize];
        f.read_exact(&mut buf).map_err(|e| DomainError::Validation(format!("{}: {len} stored bytes not readable: {e}", self.describe())))?;
        Ok(buf)
    }
}

/// Every `.abk` / `.ast` copy of a title keyed by lowercase file name (`crowds.abk`).
#[derive(Default)]
struct Sources {
    banks: BTreeMap<String, Vec<Loc>>,
    streams: BTreeMap<String, Vec<Loc>>,
    /// Places searched, for refusal text.
    searched_banks: String,
    searched_streams: String,
}

fn in_dirs(member: &str, dirs: &[&str]) -> Option<String> {
    let m = member.replace('\\', "/");
    let (dir, file) = m.rsplit_once('/')?;
    dirs.iter().any(|d| d.eq_ignore_ascii_case(dir)).then(|| file.to_ascii_lowercase())
}

fn resolve_sources(layout: &Layout, install_root: &Path) -> Result<Sources> {
    let mut s = Sources::default();
    let add = |ext: &str, key: String, loc: Loc, s: &mut Sources| {
        let map = if ext == "abk" { &mut s.banks } else { &mut s.streams };
        map.entry(key).or_default().push(loc);
    };
    for (ext, dirs) in [("abk", layout.bank_dirs), ("ast", layout.stream_dirs)] {
        for d in dirs {
            let Some(dir) = crate::assets::resolve_install_path(install_root, d) else {
                return Err(DomainError::Validation(format!("{d} escapes the install root")));
            };
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext))).collect();
            files.sort();
            for p in files {
                let key = p.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
                add(ext, key, Loc::Loose(p), &mut s);
            }
        }
    }
    let mut archives: Vec<&str> = layout.bank_archives.to_vec();
    for a in layout.stream_archives {
        if !archives.contains(a) {
            archives.push(a);
        }
    }
    for name in archives {
        let archive = install_root.join(name);
        if !archive.is_file() {
            continue;
        }
        let members = crate::gen7_speech_preview::list_members(&archive)
            .ok_or_else(|| DomainError::Validation(format!("{name}: the archive table (or its .bh sidecar) is unreadable; nothing from it is listed")))?;
        for (member, offset, size) in members {
            let lower = member.to_ascii_lowercase();
            for (ext, dirs, archs) in [("abk", layout.bank_dirs, layout.bank_archives), ("ast", layout.stream_dirs, layout.stream_archives)] {
                if !archs.contains(&name) || !lower.ends_with(&format!(".{ext}")) {
                    continue;
                }
                if let Some(key) = in_dirs(&member, dirs) {
                    add(ext, key, Loc::Member { archive: archive.clone(), archive_name: name.to_string(), member: member.clone(), offset, size }, &mut s);
                }
            }
        }
    }
    let place = |dirs: &[&str], archs: &[&str]| {
        let mut p = format!("loose {}", dirs.join(" / "));
        if !archs.is_empty() {
            p.push_str(&format!(" and {} members of {}", dirs.join(" / "), archs.join(" / ")));
        }
        p
    };
    s.searched_banks = place(layout.bank_dirs, layout.bank_archives);
    s.searched_streams = place(layout.stream_dirs, layout.stream_archives);
    Ok(s)
}

/// A bank read from its copy (or copies): decoded bytes plus provenance.
struct LoadedBank {
    loc: Loc,
    wrapper: &'static str,
    stored_sha256: String,
    bytes: Vec<u8>,
    copies: usize,
    bank: AbkBank,
}

fn unwrap_member(raw: Vec<u8>, what: &str) -> Result<(Vec<u8>, &'static str)> {
    if raw.starts_with(tdb_archive_codec::MAGIC_CHUNKZIP) {
        let b = tdb_archive_codec::decompress_chunkzip(&raw).map_err(|e| DomainError::Validation(format!("{what}: chunkzip member does not inflate: {e}")))?;
        Ok((b, "chunkzip"))
    } else if tdb_archive_codec::is_refpack(&raw) {
        let b = tdb_archive_codec::refpack_decompress(&raw).map_err(|e| DomainError::Validation(format!("{what}: RefPack member does not inflate: {e}")))?;
        Ok((b, "refpack"))
    } else {
        Ok((raw, "stored"))
    }
}

fn load_from(copies: &[Loc], name: &str) -> Result<LoadedBank> {
    let mut first: Option<LoadedBank> = None;
    for loc in copies {
        let raw = loc.read_stored()?;
        let stored_sha256 = format!("{:x}", Sha256::digest(&raw));
        let (bytes, wrapper) = if matches!(loc, Loc::Loose(_)) { (raw, "loose") } else { unwrap_member(raw, &loc.describe())? };
        match &first {
            None => {
                let bank = abk::parse_bank(&bytes).map_err(|e| DomainError::Validation(format!("{}: {e}", loc.describe())))?;
                first = Some(LoadedBank { loc: loc.clone(), wrapper, stored_sha256, bytes, copies: copies.len(), bank });
            }
            Some(f) => {
                if f.bytes != bytes {
                    return Err(DomainError::Validation(format!(
                        "{name}: two copies differ ({} and {}); which one the game loads is not established, so neither is listed",
                        f.loc.describe(),
                        loc.describe()
                    )));
                }
            }
        }
    }
    first.ok_or_else(|| DomainError::Validation(format!("{name}: no copy")))
}

/// The stream file a bank's stream records are read from: the single `<stem>.ast` copy, or — like
/// the banks in [`load_from`] — the first of several copies whose stored bytes are identical
/// (Xbox 360 NHL 09 ships each `.ast` loose and in `data0.big`, byte-identical, research R98;
/// receipt 36). Copies that differ refuse: which one the game streams is not established.
fn stream_for(sources: &Sources, bank_name: &str) -> std::result::Result<Loc, String> {
    let stem = bank_name.to_ascii_lowercase();
    let key = format!("{}.ast", stem.strip_suffix(".abk").unwrap_or(&stem));
    match sources.streams.get(&key).map(Vec::as_slice) {
        None | Some([]) => Err(format!("stream file {key} cannot be opened: it is not in {}", sources.searched_streams)),
        Some([one]) => Ok(one.clone()),
        Some(many) => {
            let differ = || {
                format!(
                    "{key} has {} copies ({}) that are not byte-identical; which one the game streams is not established, so no stream entry is framed",
                    many.len(),
                    many.iter().map(Loc::describe).collect::<Vec<_>>().join(", ")
                )
            };
            let first = many[0].read_stored().map_err(|e| e.to_string())?;
            for other in &many[1..] {
                if other.read_stored().map_err(|e| e.to_string())? != first {
                    return Err(differ());
                }
            }
            Ok(many[0].clone())
        }
    }
}

/// Header facts of a stream copy: `(description, length, id matches)`.
fn stream_facts(loc: &Loc, bank_ast_id: u32) -> (String, Option<u64>, bool) {
    let Ok((mut f, base, len)) = loc.extent() else {
        return (format!("{} (unreadable)", loc.describe()), None, false);
    };
    let mut head = [0u8; 8];
    let ok = f.seek(SeekFrom::Start(base)).is_ok() && len >= 8 && f.read_exact(&mut head).is_ok() && abk::ast_id(&head).ok() == Some(bank_ast_id);
    (loc.describe(), Some(len), ok)
}

/// One bank of the listing (header facts only; the tree is read on request).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7SfxBankSummary {
    pub name: String,
    /// The loose path, or `archive:member` for an archive member.
    pub path: String,
    pub len: u64,
    pub platform: Option<String>,
    pub records: usize,
    pub stream_records: usize,
    pub ram_records: usize,
    pub lists: usize,
    pub names: usize,
    /// The bank's stream file when it has a stream side: where, length, id match.
    pub ast: Option<String>,
    pub ast_len: Option<u64>,
    pub ast_id_matches: Option<bool>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7SfxBankList {
    pub game_id: String,
    pub platform: String,
    pub population: String,
    /// Where the banks were looked for (loose directory and / or archive members).
    pub dir: String,
    pub banks: Vec<Gen7SfxBankSummary>,
    pub notes: Vec<String>,
}

fn empty_summary(name: String, path: String, len: u64, problem: String) -> Gen7SfxBankSummary {
    Gen7SfxBankSummary { name, path, len, platform: None, records: 0, stream_records: 0, ram_records: 0, lists: 0, names: 0, ast: None, ast_len: None, ast_id_matches: None, problem: Some(problem) }
}

fn display_name(copies: &[Loc], key: &str) -> String {
    copies
        .first()
        .map(|l| match l {
            Loc::Loose(p) => p.file_name().unwrap().to_string_lossy().into_owned(),
            Loc::Member { member, .. } => member.rsplit('/').next().unwrap_or(member).to_string(),
        })
        .unwrap_or_else(|| key.to_string())
}

/// Every `.abk` of a measured title — loose or from the title's own archive members — parsed and
/// summarised.
pub fn list_banks(game_id: &str, platform: crate::Platform, install_root: &Path) -> Result<Gen7SfxBankList> {
    let layout = layout(game_id, platform)?;
    let sources = resolve_sources(&layout, install_root)?;
    let mut banks = Vec::with_capacity(sources.banks.len());
    for (key, copies) in &sources.banks {
        let name = display_name(copies, key);
        let path = copies.iter().map(Loc::describe).collect::<Vec<_>>().join(" = ");
        match load_from(copies, &name) {
            Ok(l) => {
                let (ast, ast_len, ast_id_matches) = if l.bank.ast_id == 0 {
                    (None, None, None)
                } else {
                    match stream_for(&sources, &name) {
                        Ok(loc) => {
                            let (d, len, ok) = stream_facts(&loc, l.bank.ast_id);
                            (Some(d), len, Some(ok))
                        }
                        Err(why) => (Some(format!("({why})")), None, Some(false)),
                    }
                };
                banks.push(Gen7SfxBankSummary {
                    name,
                    path,
                    len: l.bytes.len() as u64,
                    platform: Some(l.bank.platform.label().to_string()),
                    records: l.bank.record_count(),
                    stream_records: l.bank.stream_records(),
                    ram_records: l.bank.ram_records(),
                    lists: l.bank.lists.len(),
                    names: l.bank.distinct_names(),
                    ast,
                    ast_len,
                    ast_id_matches,
                    problem: None,
                });
            }
            Err(e) => banks.push(empty_summary(name, path, 0, e.to_string())),
        }
    }
    banks.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    let dir = if layout.bank_archives.is_empty() {
        crate::assets::resolve_install_path(install_root, layout.bank_dirs[0]).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
    } else {
        format!("{} — {}", install_root.display(), sources.searched_banks)
    };
    Ok(Gen7SfxBankList {
        game_id: game_id.to_string(),
        platform: platform_label(platform).to_string(),
        population: layout.population.to_string(),
        dir,
        banks,
        notes: vec![
            "Listing only: bank headers, record counts and the stream file's id check (R82 / R89 / R98). Which bank the game loads for an event is the loader's business (`na_en_loadfile.xml`, R85), not established here.".into(),
            layout.evidence.into(),
            "Replacement is not supported for any bank or stream: no encoder, and the bank tree's S / X words are unidentified (R94) — nothing here writes.".into(),
        ],
    })
}

/// One entry of a picker list, resolved to its record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7SfxEntry {
    pub record: u16,
    pub weight: u8,
    /// "ram" / "ram-loop" / "stream" / "stream-loop".
    pub kind: String,
    pub codec: String,
    pub codec_code: u8,
    pub sample_rate: u32,
    pub channel_config: u8,
    pub sub_streams: Option<usize>,
    pub num_samples: u32,
    pub stream_offset: Option<u64>,
    /// Stream records: blocks walked / bytes / whether the sample words summed to the record's count.
    pub blocks: Option<usize>,
    pub stream_bytes: Option<u64>,
    pub framing: Option<String>,
    /// Unknown words, carried as read (R94) — behind disclosure only.
    pub x: u16,
    pub s: u32,
    pub previewable: bool,
    pub preview_refusal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7SfxPicker {
    pub owners: Vec<String>,
    pub owner_class: Option<String>,
    pub list_offset: u32,
    pub empty_slots: usize,
    pub entries: Vec<Gen7SfxEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7SfxBankTree {
    pub game_id: String,
    pub platform: String,
    pub bank: String,
    /// The loose path, or `archive:member`.
    pub bank_path: String,
    /// Decoded bank length.
    pub bank_len: u64,
    /// SHA-256 of the decoded bank bytes.
    pub bank_sha256: String,
    /// "loose", or the member's stored form: "stored" / "refpack" / "chunkzip".
    pub bank_wrapper: String,
    /// SHA-256 of the bytes as stored (= `bank_sha256` for loose and stored copies).
    pub bank_stored_sha256: String,
    /// Identical copies found (loose + members); differing copies refuse.
    pub bank_copies: usize,
    pub bank_platform: String,
    pub ast_path: Option<String>,
    pub ast_len: Option<u64>,
    pub ast_id: u32,
    pub ast_id_matches: Option<bool>,
    pub records: usize,
    pub stream_records: usize,
    pub ram_records: usize,
    pub lists: usize,
    pub names: usize,
    pub handler_nodes: u16,
    pub pickers: Vec<Gen7SfxPicker>,
    pub notes: Vec<String>,
}

fn class_label(c: Option<u8>) -> Option<String> {
    c.map(|c| match c {
        0 => "sound (_snd)".to_string(),
        1 => "handler (h_)".to_string(),
        2 => "message (_msg)".to_string(),
        other => format!("class {other}"),
    })
}

fn load_bank(layout: &Layout, install_root: &Path, bank_name: &str) -> Result<(Sources, LoadedBank)> {
    if bank_name.contains(['/', '\\']) || !bank_name.to_ascii_lowercase().ends_with(".abk") {
        return Err(DomainError::Validation(format!("`{bank_name}` is not a bank file name")));
    }
    let sources = resolve_sources(layout, install_root)?;
    let copies = sources
        .banks
        .get(&bank_name.to_ascii_lowercase())
        .ok_or_else(|| DomainError::Validation(format!("{bank_name}: not in {}", sources.searched_banks)))?;
    let l = load_from(copies, bank_name)?;
    Ok((sources, l))
}

/// The folder of the running executable (where a `helpers` folder may hold the XMA2 helper).
fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf))
}

/// Titles with a research R92 vector of a **non-looping, single-stream** (channel config 0 / 1)
/// EA-XMA `.ast` stream record decoded to exactly its declared count (R92 vectors F / G on NHL 15,
/// I2 on NHL 10, I4 on Legacy, I6 on NHL 12). Receipt 33.
const XMA_STREAM_TITLES: &[&str] = &["nhl09", "nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"];

/// Receipt 53 (research 2026-09-28 handoff 06, R140): the stream shapes research decoded exactly on **every** record of
/// each title's banks — `(looping, channel config) → titles`. NHL 11 / 13 / 14 were swept in full, NHL 12 as the control
/// (same result); NHL 15's looping channel-config-3 record is R92 vector H (receipt 33).
// Receipt 56: research 2026-09-28 handoff 06 §4 addendum (R140b) swept NHL 10 / 15 / Legacy — every stream record exact.
const XMA_STREAM_SHAPES: &[((bool, u8), &[&str])] = &[
    ((true, 1), &["nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"]),
    ((false, 3), &["nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"]),
    ((true, 3), &["nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"]),
    ((false, 5), &["nhl13", "nhl14", "nhl15", "nhllegacy"]),
];

/// The packet-header reading per title: NHL 09's constant-header variant (R98b; R131 — the NHL 10+
/// header check would refuse 47 NHL 09 records that decode exactly), the checked reading elsewhere.
fn header_rule(game_id: &str) -> tdb_snd::eaxma::HeaderRule {
    let game_id = xma_title(game_id);
    if game_id == "nhl09" {
        tdb_snd::eaxma::HeaderRule::Nhl09Variant
    } else if XMA_RAM_RESIDUAL_TITLES.contains(&game_id) {
        // Research 2026-09-28 handoff 05 X1: the (0, 0x7FFF) "no frame begins here" packet is valid in these
        // titles' ram records (80–103 sub-streams per title decode exactly with it kept).
        tdb_snd::eaxma::HeaderRule::CheckedNoFrameMarker
    } else {
        tdb_snd::eaxma::HeaderRule::Checked
    }
}

/// Titles whose X360 ram-record residual shapes research measured (2026-09-28 handoff 05, R139 / R139b / R139c):
/// the no-frame packet (X1), two-segment intro + loop stereo records (X2), single-segment looping stereo /
/// four-channel records (X3), four- / six-channel `w0` chains (X4). Every segment / chunk decoded to exactly its
/// declared count with the pinned helper. Legacy Recompiled uses the Legacy entry through `xma_title` (receipt 59).
const XMA_RAM_RESIDUAL_TITLES: &[&str] = &["nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"];

/// Stereo chunks per ram segment by channel config (X4): 0 / 1 → one stream, 3 → two, 5 → three.
fn ram_chunks(channel_config: u8) -> Option<usize> {
    match channel_config {
        0 | 1 => Some(1),
        3 => Some(2),
        5 => Some(3),
        _ => None,
    }
}
/// Titles with an R92 vector of a **looping, channel config 3** EA-XMA stream record (vector H, NHL 15
/// `crowds` record 0: stub + body per stereo pair, each at its declared count). Receipt 33.
const XMA_LOOP_PAIR_TITLES: &[&str] = &["nhl15"];

/// The EA-XMA route (receipts 30 / 33): admitted only where research measured it — Xbox 360 (not
/// Recompiled); single-segment, non-looping ram records with channel config 0 / 1 (R84); `.ast`
/// stream records of the shapes and titles in [`XMA_STREAM_TITLES`] / [`XMA_LOOP_PAIR_TITLES`]
/// (R92) — and only when an XMA2 helper is installed; everything else stays framed with the reason
/// named.
/// The title whose X360 measurements apply: Legacy Recompiled's banks are the X360 Legacy bytes (R128 / R138, receipt 59).
fn xma_title(game_id: &str) -> &str {
    if matches!(game_id, "legacyrecomp" | "nhllegacy-recompiled") {
        "nhllegacy"
    } else {
        game_id
    }
}

/// The label every Recompiled preview carries (research handoff 04 §6.2).
pub const RECOMP_PREVIEW_NOTE: &str = "Legacy Recompiled: these archive-member bytes are the Xbox 360 Legacy files (R128 / R138), decoded by the X360 Legacy rules; what the recompiled game plays is not observed.";

fn xma_gate(game_id: &str, rec: &abk::Record, platform: crate::Platform) -> (bool, Option<String>) {
    let what = format!("{} (Xbox 360 EA-XMA, codec 0x{:02x})", rec.header.codec_label(), rec.header.codec);
    let game_id = xma_title(game_id);
    if platform != crate::Platform::Xbox && !(platform == crate::Platform::Recomp && game_id == "nhllegacy") {
        return (false, Some(format!("{what}: framed only — no measured EA-XMA vector on this source (R84 covers the Xbox 360 discs)")));
    }
    if rec.header.codec != tdb_snd::eaxma::CODEC {
        return (false, Some(format!("{what}: no diagnostic route for this codec")));
    }
    match &rec.body {
        RecordBody::Stream { loop_words } => {
            let shape = (loop_words.is_some(), rec.header.channel_config);
            let measured = match shape {
                (false, 0 | 1) => XMA_STREAM_TITLES.contains(&game_id),
                (true, 3) if XMA_LOOP_PAIR_TITLES.contains(&game_id) => true,
                _ => XMA_STREAM_SHAPES.iter().any(|(k, t)| *k == shape && t.contains(&game_id)),
            };
            if !measured {
                return (
                    false,
                    Some(format!(
                        "{what}: {} stream record with channel config {} — no measured EA-XMA stream vector of this shape on {game_id} (R92 measured non-looping mono / stereo streams on NHL 10 / 12 / 15 / Legacy and one looping channel-config-3 record on NHL 15; R140 / R140b every stream record of NHL 10 → 15 and Legacy); framed only",
                        if loop_words.is_some() { "looping" } else { "non-looping" },
                        rec.header.channel_config
                    )),
                );
            }
        }
        // Research 2026-09-28 handoff 05 (receipt 50): on NHL 10 → Legacy, single-segment looping stereo / four-channel
        // records (X3; played once), two-segment looping stereo records whose loop starts where segment 0 ends (X2;
        // intro + body), and four- / six-channel chains (X4; one stereo pair at a time) decode exactly.
        RecordBody::Ram { segments, loop_start } if XMA_RAM_RESIDUAL_TITLES.contains(&game_id) => {
            let cc = rec.header.channel_config;
            let ok = match (segments.len(), rec.header.looped, cc) {
                (1, false, 0 | 1 | 3 | 5) => true,
                (1, true, 1 | 3) => loop_start.unwrap_or(0) == 0,
                (2, true, 1) => *loop_start == Some(segments[0].samples),
                _ => false,
            };
            if !ok {
                return (
                    false,
                    Some(format!(
                        "{what}: {} ram segment(s), {}, channel config {cc} — not a shape research measured on this title (handoff 05: 1-segment mono / stereo / 4 / 6 channels, 1-segment looping stereo / 4 channels, 2-segment looping stereo with the loop at segment 0's end)",
                        segments.len(),
                        if rec.header.looped { "looping" } else { "not looping" }
                    )),
                );
            }
        }
        // NHL 09: R131 decoded 176 / 177 looping in-bank segments to their exact count (played once
        // here); on other titles looping ram records are not measured.
        RecordBody::Ram { .. } if rec.header.looped && game_id != "nhl09" => return (false, Some(format!("{what}: looping ram record — loop boundaries are not measured for EA-XMA on {game_id}"))),
        RecordBody::Ram { segments, .. } if segments.len() != 1 => return (false, Some(format!("{what}: {} ram segments — only single-segment records are measured (R84)", segments.len()))),
        RecordBody::Ram { .. } if !matches!(rec.header.channel_config, 0 | 1) => {
            return (false, Some(format!("{what}: channel config {} — only mono / stereo ram records are measured", rec.header.channel_config)));
        }
        _ => {}
    }
    match tdb_snd::helper::locate(exe_dir().as_deref()) {
        Ok(_) => (true, None),
        Err(e) => (false, Some(format!("{what}: {e}"))),
    }
}

/// Whether a record's codec / channel configuration has a diagnostic decode route here.
fn preview_gate(game_id: &str, bank: &AbkBank, rec: &abk::Record, platform: crate::Platform) -> (bool, Option<String>) {
    if bank.platform == BankPlatform::Xbox360 {
        return xma_gate(game_id, rec, platform);
    }
    if rec.header.codec != 0x05 {
        return (false, Some(format!("codec 0x{:02x} ({}) has no diagnostic route here (only EALayer3 v1 does)", rec.header.codec, rec.header.codec_label())));
    }
    // Channel configs 3 / 5 preview per sub-stream (inferred de-interleave) and stream loops as
    // stub + body since 26 Sep; the tree still refuses an entry whose loop body does not frame.
    match abk::sub_streams(rec.header.channel_config) {
        Some(_) => (true, None),
        None => (false, Some(format!("channel config {} is not a measured value (0 / 1 / 3 / 5)", rec.header.channel_config))),
    }
}

/// The named picker tree of one bank with every entry resolved to its record and, for stream
/// records, framed in the bank's stream file.
pub fn bank_tree(game_id: &str, platform: crate::Platform, install_root: &Path, bank_name: &str) -> Result<Gen7SfxBankTree> {
    let layout = layout(game_id, platform)?;
    let (sources, l) = load_bank(&layout, install_root, bank_name)?;
    let bank = &l.bank;
    let stream = if bank.ast_id == 0 { None } else { Some(stream_for(&sources, bank_name)) };
    let (ast_path, ast_len, ast_id_matches) = match &stream {
        None => (None, None, None),
        Some(Ok(loc)) => {
            let (d, len, ok) = stream_facts(loc, bank.ast_id);
            (Some(d), len, Some(ok))
        }
        Some(Err(why)) => (Some(format!("({why})")), None, Some(false)),
    };
    let mut pickers = Vec::with_capacity(bank.lists.len());
    for list in &bank.lists {
        let mut entries = Vec::with_capacity(list.entries.len());
        for e in &list.entries {
            let Some(rec) = bank.record(e.index) else { continue };
            let (mut previewable, mut refusal) = preview_gate(game_id, bank, rec, platform);
            let (mut blocks, mut stream_bytes, mut framing) = (None, None, None);
            if let Some(off) = e.stream_offset {
                let framed = match &stream {
                    Some(Ok(loc)) => frame_stream(loc, bank.ast_id, off as u64, rec),
                    Some(Err(why)) => Err(DomainError::Validation(why.clone())),
                    None => frame_stream(&Loc::Loose(PathBuf::new()), 0, off as u64, rec),
                };
                match framed {
                    Ok(Framed { main: s, body }) => {
                        blocks = Some(s.blocks.len() + body.as_ref().map_or(0, |b| b.blocks.len()));
                        stream_bytes = Some(s.end - s.start + body.as_ref().map_or(0, |b| b.end - b.start));
                        framing = Some(match (&rec.body, &body) {
                            (RecordBody::Stream { loop_words: Some((a, b)) }, Some(bd)) => format!("loop stub ({a} samples, {b} B) + body at {} ({} blocks, {} samples) framed; stub + body = the record's count", bd.start, bd.blocks.len(), bd.samples),
                            _ => "framed; block sample words sum to the record's count".to_string(),
                        });
                    }
                    Err(e) => {
                        let e = e.to_string();
                        framing = Some(format!("framing refused: {e}"));
                        previewable = false;
                        refusal = Some(format!("framing refused: {e}"));
                    }
                }
            }
            entries.push(Gen7SfxEntry {
                record: e.index,
                weight: e.weight,
                kind: rec.kind().to_string(),
                codec: rec.header.codec_label().to_string(),
                codec_code: rec.header.codec,
                sample_rate: rec.header.sample_rate,
                channel_config: rec.header.channel_config,
                sub_streams: abk::sub_streams(rec.header.channel_config),
                num_samples: rec.header.num_samples,
                stream_offset: e.stream_offset.map(u64::from),
                blocks,
                stream_bytes,
                framing,
                x: e.x,
                s: e.s,
                previewable,
                preview_refusal: refusal,
            });
        }
        pickers.push(Gen7SfxPicker { owners: list.owners.clone(), owner_class: class_label(list.owner_class), list_offset: list.at, empty_slots: list.empty_slots, entries });
    }
    Ok(Gen7SfxBankTree {
        game_id: game_id.to_string(),
        platform: platform_label(platform).to_string(),
        bank: bank_name.to_string(),
        bank_path: l.loc.describe(),
        bank_len: l.bytes.len() as u64,
        bank_sha256: format!("{:x}", Sha256::digest(&l.bytes)),
        bank_wrapper: l.wrapper.to_string(),
        bank_stored_sha256: l.stored_sha256.clone(),
        bank_copies: l.copies,
        bank_platform: bank.platform.label().to_string(),
        ast_path,
        ast_len,
        ast_id: bank.ast_id,
        ast_id_matches,
        records: bank.record_count(),
        stream_records: bank.stream_records(),
        ram_records: bank.ram_records(),
        lists: bank.lists.len(),
        names: bank.distinct_names(),
        handler_nodes: bank.handler_nodes,
        pickers,
        notes: vec![
            "Pickers are the named nodes whose extent reaches each sample list (R89: `_msg` sample pickers, `_snd` parameter nodes, `h_` handlers); the weight is a pick weight by the owner's semantics only. Node bodies are not decoded.".into(),
            "Stream entries are framed from their `.ast` start to the terminal block and the block sample words are summed against the record (R91); loop records = stub + body. The decode route (EALayer3 v1, one sub-stream) produces the R92 candidate construction and is labelled diagnostic; whether it is what the console renders is not established.".into(),
            layout.evidence.into(),
            "X / S are unidentified words carried as read (R94) — shown behind disclosure, never rewritten. Replacement is not supported.".into(),
        ],
    })
}

/// Frame one stream record from its start in the bank's stream file; loop records walk the stub
/// and check its words.
///
/// The stream file's identity is checked on the **same handle** the stream bytes are read from
/// (review 2026-09-26 F1): the `S10S` magic and the id word must equal the bank's header word
/// 0x2c (R89 T5). A missing, malformed or mismatched stream file — including one replaced after
/// the bank was listed — refuses before any framing or decode. An archive member is read in place
/// and the walk is bounded to the member's stored extent; a wrapped member is refused.
/// A framed stream entry: the referenced stream and, for a loop record, the body that follows it.
struct Framed {
    main: abk::AstStream,
    body: Option<abk::AstStream>,
}

fn frame_stream(loc: &Loc, bank_ast_id: u32, start: u64, rec: &abk::Record) -> Result<Framed> {
    if bank_ast_id == 0 {
        return Err(DomainError::Validation("the bank declares no stream file (header word 0x2c is 0), so a stream record cannot be read".into()));
    }
    let what = loc.describe();
    let (mut f, base, len) = loc.extent().map_err(|e| DomainError::Validation(format!("stream file {what} cannot be opened: {e}")))?;
    let mut head = [0u8; 8];
    if len < 8 || f.seek(SeekFrom::Start(base)).is_err() || f.read_exact(&mut head).is_err() {
        return Err(DomainError::Validation(format!("stream file {what} is shorter than its 8-byte header")));
    }
    if matches!(loc, Loc::Member { .. }) && (head.starts_with(tdb_archive_codec::MAGIC_CHUNKZIP) || tdb_archive_codec::is_refpack(&head)) {
        return Err(DomainError::Validation(format!("stream member {what} is stored wrapped (RefPack / chunkzip); R98 found every stream member stored, so it is not inflated or framed")));
    }
    let id = abk::ast_id(&head).map_err(|e| DomainError::Validation(format!("stream file {what}: {e}")))?;
    if id != bank_ast_id {
        return Err(DomainError::Validation(format!(
            "stream file {what} has id 0x{id:08x}, the bank names 0x{bank_ast_id:08x} — it is not this bank's stream file; nothing is framed or decoded"
        )));
    }
    if start < 0x40 {
        return Err(DomainError::Validation(format!("stream start {start} lies inside the stream file header")));
    }
    let s = abk::walk_stream_in(&mut f, base, len, start).map_err(|e| DomainError::Validation(e.to_string()))?;
    match &rec.body {
        RecordBody::Stream { loop_words: Some((a, b)) } => {
            let stub = &s.blocks[0];
            if s.blocks.len() != 1 || stub.samples != *a || stub.size != *b {
                return Err(DomainError::Validation(format!("loop stub at {start}: ({} samples, {} B in {} block(s)) ≠ record words ({a}, {b})", stub.samples, stub.size, s.blocks.len())));
            }
            // R91 §1 (PS3): the body is the next stream, at the stub's end, on the same checked handle.
            let body = abk::walk_stream_in(&mut f, base, len, s.end).map_err(|e| DomainError::Validation(format!("loop body at {}: {e}", s.end)))?;
            if *a as u64 + body.samples != rec.header.num_samples as u64 {
                return Err(DomainError::Validation(format!("loop: stub {a} + body {} samples ≠ the record's {}", body.samples, rec.header.num_samples)));
            }
            Ok(Framed { main: s, body: Some(body) })
        }
        _ => {
            if s.samples != rec.header.num_samples as u64 {
                return Err(DomainError::Validation(format!("stream at {start}: block sample words sum to {}, the record says {}", s.samples, rec.header.num_samples)));
            }
            Ok(Framed { main: s, body: None })
        }
    }
}

/// The diagnostic decode of one entry (PS3 EALayer3 v1, channel config 0 / 1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7SfxPreview {
    pub game_id: String,
    pub platform: String,
    pub bank: String,
    pub bank_sha256: String,
    pub record: u16,
    pub kind: String,
    /// Channel config 3 / 5: which stereo sub-stream (0-based) of `sub_streams` this preview is.
    pub sub_stream: Option<u16>,
    pub sub_streams: u16,
    /// Stream loops: the loop point (the stub's `a` samples) inside the preview.
    pub loop_start: Option<u32>,
    pub owners: Vec<String>,
    pub source: String,
    pub source_sha256: String,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub num_samples: u32,
    pub blocks: usize,
    pub granules: usize,
    pub rebuilt_frames: usize,
    pub decoded_frames_per_channel: usize,
    pub label: String,
    pub wav: Vec<u8>,
    pub notes: Vec<String>,
}

/// The EA-XMA diagnostic preview of one gated ram record through the installed XMA2 helper (explicit
/// path, fixed arguments, bounded; `tdb_snd::helper`). The decoded count must equal the record's.
fn xma_preview(game_id: &str, platform: crate::Platform, bank_name: &str, record: u16, l: &LoadedBank, rec: &abk::Record, sub_stream: Option<u16>) -> Result<Gen7SfxPreview> {
    let err = |e: String| DomainError::Validation(format!("{bank_name} record {record}: {e}"));
    let RecordBody::Ram { segments, loop_start } = &rec.body else { return Err(err("not a ram record".into())) };
    // Receipt 50 (handoff 05): each segment decodes on its own (never splicing packets); a four- / six-channel
    // segment is a chain of stereo chunks and one chunk (`sub_stream`, default 0) is decoded per segment.
    let chunks = ram_chunks(rec.header.channel_config).ok_or_else(|| err(format!("channel config {} is not a measured value", rec.header.channel_config)))?;
    let k = sub_stream.unwrap_or(0) as usize;
    if k >= chunks {
        return Err(err(format!("stereo pair {} of {chunks} does not exist", k + 1)));
    }
    let ch: u16 = if chunks > 1 { 2 } else { rec.header.channels() as u16 };
    let helper = tdb_snd::helper::locate(exe_dir().as_deref()).map_err(|e| err(e.to_string()))?;
    let mut pcm: Vec<i16> = Vec::new();
    let (mut packets, mut frames) = (0usize, 0u32);
    let mut seg_bytes: Vec<u8> = Vec::new();
    for (si, seg) in segments.iter().enumerate() {
        let part: &[u8] = if chunks == 1 {
            &seg.bytes
        } else {
            let c = tdb_snd::eaxma::segment_chunks(&seg.bytes).map_err(|e| err(format!("segment {si}: {e}")))?;
            if c.len() != chunks {
                return Err(err(format!("segment {si}: {} chunks, channel config {} needs {chunks}", c.len(), rec.header.channel_config)));
            }
            c[k]
        };
        let p = tdb_snd::eaxma::raw_segment_packets_with(part, header_rule(game_id)).map_err(|e| err(format!("segment {si}: EA-XMA packets refused: {e}")))?;
        let riff = tdb_snd::eaxma::riff_xma2(&p.packets, seg.samples, ch, rec.header.sample_rate);
        let out = tdb_snd::helper::decode_xma2(&helper, &riff, tdb_snd::helper::Limits::default(), None).map_err(|e| err(e.to_string()))?;
        let per = out.len() / ch as usize;
        if per as u32 != seg.samples {
            return Err(err(format!("segment {si}: the helper decoded {per} samples per channel, the segment declares {} — refused", seg.samples)));
        }
        pcm.extend_from_slice(&out);
        packets += p.packet_count;
        frames += p.frame_count_sum;
        seg_bytes.extend_from_slice(part);
    }
    let per_channel = pcm.len() / ch as usize;
    if per_channel as u32 != rec.header.num_samples {
        return Err(err(format!("the helper decoded {per_channel} samples per channel, the record declares {} — refused", rec.header.num_samples)));
    }
    let loop_at = (rec.header.looped && segments.len() == 2).then(|| loop_start.unwrap_or(0));
    let mut out = Gen7SfxPreview {
        game_id: game_id.to_string(),
        platform: platform_label(platform).to_string(),
        bank: bank_name.to_string(),
        bank_sha256: format!("{:x}", Sha256::digest(&l.bytes)),
        record,
        kind: rec.kind().to_string(),
        sub_stream: (chunks > 1).then_some(k as u16),
        sub_streams: chunks as u16,
        loop_start: loop_at,
        owners: l.bank.lists_naming(record).iter().flat_map(|x| x.owners.iter().cloned()).collect(),
        source: format!("{} record {record} (ram, {} segment(s) inline, {packets} XMA2 packets)", l.loc.describe(), segments.len()),
        source_sha256: format!("{:x}", Sha256::digest(&seg_bytes)),
        codec: rec.header.codec_label().to_string(),
        sample_rate: rec.header.sample_rate,
        channels: ch as u32,
        num_samples: rec.header.num_samples,
        blocks: 1,
        granules: 0,
        rebuilt_frames: 0,
        decoded_frames_per_channel: per_channel,
        label: tdb_snd::eaxma::LABEL.to_string(),
        wav: tdb_snd::wav::encode_pcm16(rec.header.sample_rate, ch, &pcm),
        notes: vec![
            format!("{packets} XMA2 packets ({frames} frames) wrapped in RIFF XMA2 and decoded by the helper at {}; {} samples per channel, equal to the record's count.", helper.display(), per_channel),
            match (segments.len(), rec.header.looped, chunks) {
                (2, _, _) => format!("Intro + loop: segment 0 ({} samples) and segment 1 ({} samples) decoded separately and joined; the loop starts at sample {} (played once).", segments[0].samples, segments[1].samples, loop_at.unwrap_or(0)),
                (1, true, _) => "The whole record loops; it is played once.".into(),
                _ => String::new(),
            },
            if chunks > 1 { format!("Stereo pair {} of {chunks} of a {}-channel record (one XMA sub-stream per pair); which speakers a pair feeds is not established.", k + 1, chunks * 2) } else { String::new() },
            if game_id == "nhl09" {
                "Research R131 decoded every NHL 09 in-bank segment through this route: 1 983 / 1 985 at exactly their declared counts (the two short ones refuse); NHL 09's packet headers are a constant variant and are not read as frame counts. A looping record is played once. Counts are not audible fidelity.".into()
            } else {
                "Research R84 measured this route on Xbox 360 NHL 10 → Legacy ram entries (exact counts on every vector). Counts are not audible fidelity.".into()
            },
            "Diagnostic only: not game-verified, not listened to by the developer; replacement is not supported.".into(),
        ],
    };
    out.notes.retain(|n| !n.is_empty());
    Ok(out)
}

/// The EA-XMA `.ast` stream route (R92; receipt 33): the stream is framed on the checked handle
/// (identity, extent, block sums), each block's sub-blocks are length- and header-checked, the
/// packets of the one stream — or of the chosen stereo pair of a channel-config-3 record — are
/// wrapped as RIFF XMA2 and decoded by the helper. A loop record decodes its stub and its body
/// separately, each to its own declared count, and joins them once (no repetition). The decoded
/// count must equal the declared count, or nothing is returned.
#[allow(clippy::too_many_arguments)]
fn xma_stream_preview(game_id: &str, platform: crate::Platform, bank_name: &str, record: u16, sources: &Sources, l: &LoadedBank, rec: &abk::Record, sub_stream: Option<u16>) -> Result<Gen7SfxPreview> {
    let err = |e: String| DomainError::Validation(format!("{bank_name} record {record}: {e}"));
    let bank = &l.bank;
    let RecordBody::Stream { loop_words } = &rec.body else { return Err(err("not a stream record".into())) };
    let n = abk::sub_streams(rec.header.channel_config).ok_or_else(|| err(format!("channel config {} is not a measured value", rec.header.channel_config)))?;
    let part = match (n, sub_stream) {
        (1, None | Some(0)) => None,
        (1, Some(k)) => return Err(err(format!("one stream only; sub-stream {} does not exist", k + 1))),
        (n, None) => return Err(err(format!("channel config {} carries {n} XMA streams (stereo pairs) — choose one (1–{n})", rec.header.channel_config))),
        (n, Some(k)) if k as usize >= n => return Err(err(format!("sub-stream {} of {n} does not exist", k + 1))),
        (_, Some(k)) => Some(k as usize),
    };
    let entry = bank.lists.iter().flat_map(|l| l.entries.iter()).find(|e| e.index == record).ok_or_else(|| err("no list entry names it".into()))?;
    let off = entry.stream_offset.ok_or_else(|| err("stream record without a stream offset".into()))? as u64;
    let loc = stream_for(sources, bank_name).map_err(DomainError::Validation)?;
    let Framed { main: s, body } = frame_stream(&loc, bank.ast_id, off, rec)?;
    let (stub, data, what) = match (loop_words, body) {
        (Some((a, _)), Some(bd)) => (Some((*a, s.blocks[0].payload.clone())), bd, format!("loop stub at {off} + body at {}", s.end)),
        _ => (None, s, format!("stream at {off}")),
    };
    let ch: u16 = if n == 1 { rec.header.channels() as u16 } else { 2 };
    let helper = tdb_snd::helper::locate(exe_dir().as_deref()).map_err(|e| err(e.to_string()))?;
    let decode = |payloads: &[&[u8]], samples: u32, what: &str| -> Result<(Vec<i16>, usize, u32)> {
        let p = tdb_snd::eaxma::block_packets(payloads, n, part, true, header_rule(game_id)).map_err(|e| err(format!("{what}: EA-XMA packets refused: {e}")))?;
        let riff = tdb_snd::eaxma::riff_xma2(&p.packets, samples, ch, rec.header.sample_rate);
        let pcm = tdb_snd::helper::decode_xma2(&helper, &riff, tdb_snd::helper::Limits::default(), None).map_err(|e| err(format!("{what}: {e}")))?;
        let got = pcm.len() / ch as usize;
        if got as u32 != samples {
            return Err(err(format!("{what}: the helper decoded {got} samples per channel, {samples} are declared — refused")));
        }
        Ok((pcm, p.packet_count, p.frame_count_sum))
    };
    let mut pcm = Vec::new();
    let (mut packets, mut frames) = (0usize, 0u32);
    let body_samples = match &stub {
        Some((a, stub_bytes)) => {
            let (p, k, f) = decode(&[stub_bytes.as_slice()], *a, "loop stub")?;
            pcm.extend(p);
            packets += k;
            frames += f;
            rec.header.num_samples - a
        }
        None => rec.header.num_samples,
    };
    let payloads: Vec<&[u8]> = data.blocks.iter().map(|b| b.payload.as_slice()).collect();
    let (p, k, f) = decode(&payloads, body_samples, if stub.is_some() { "loop body" } else { "stream" })?;
    pcm.extend(p);
    packets += k;
    frames += f;
    let per_channel = pcm.len() / ch as usize;
    let raw: Vec<u8> = stub.iter().flat_map(|(_, p)| p.iter().copied()).chain(data.blocks.iter().flat_map(|b| b.payload.iter().copied())).collect();
    Ok(Gen7SfxPreview {
        game_id: game_id.to_string(),
        platform: platform_label(platform).to_string(),
        bank: bank_name.to_string(),
        bank_sha256: format!("{:x}", Sha256::digest(&l.bytes)),
        record,
        kind: rec.kind().to_string(),
        sub_stream: part.map(|k| k as u16),
        sub_streams: n as u16,
        loop_start: stub.as_ref().map(|(a, _)| *a),
        owners: bank.lists_naming(record).iter().flat_map(|x| x.owners.iter().cloned()).collect(),
        source: format!("{} {what} ({} blocks, {} B)", loc.describe(), data.blocks.len(), data.end - data.start),
        source_sha256: format!("{:x}", Sha256::digest(&raw)),
        codec: rec.header.codec_label().to_string(),
        sample_rate: rec.header.sample_rate,
        channels: ch as u32,
        num_samples: rec.header.num_samples,
        blocks: data.blocks.len() + stub.is_some() as usize,
        granules: 0,
        rebuilt_frames: 0,
        decoded_frames_per_channel: per_channel,
        label: tdb_snd::eaxma::LABEL.to_string(),
        wav: tdb_snd::wav::encode_pcm16(rec.header.sample_rate, ch, &pcm),
        notes: vec![
            format!(
                "{packets} XMA2 packets ({frames} frames){} wrapped in RIFF XMA2 and decoded by the helper at {}; {per_channel} samples per channel = the record's count{}.",
                match part {
                    Some(k) => format!(" of stereo pair {} of {n}", k + 1),
                    None => String::new(),
                },
                helper.display(),
                if stub.is_some() { " (loop stub and body decoded separately, each to its own count, joined once)" } else { "" }
            ),
            if game_id == "nhl09" {
                "Research R131 decoded all 471 non-loop mono / stereo NHL 09 stream entries through this route to exactly their declared counts (constant packet-header variant, not read as frame counts). Counts are not audible fidelity.".into()
            } else {
                "Research R92 measured `.ast` EA-XMA streams of this shape on this title (exact counts, receipts 30 / 33); a stereo pair of a multichannel record is a diagnostic selection, not how the console mixes it. Counts are not audible fidelity.".into()
            },
            "Diagnostic only: not game-verified, not listened to by the developer; replacement is not supported.".into(),
        ],
    })
}

pub fn preview_entry(game_id: &str, platform: crate::Platform, install_root: &Path, bank_name: &str, record: u16) -> Result<Gen7SfxPreview> {
    preview_part(game_id, platform, install_root, bank_name, record, None)
}

/// The diagnostic decode of one entry, or of one stereo sub-stream (`sub_stream`, 0-based) of a
/// channel-config 3 / 5 entry. A multi-stream entry without a sub-stream refuses in words.
pub fn preview_part(game_id: &str, platform: crate::Platform, install_root: &Path, bank_name: &str, record: u16, sub_stream: Option<u16>) -> Result<Gen7SfxPreview> {
    let layout = layout(game_id, platform)?;
    let (sources, l) = load_bank(&layout, install_root, bank_name)?;
    let bank = &l.bank;
    let rec = bank
        .record(record)
        .ok_or_else(|| DomainError::Validation(format!("{bank_name}: record {record} is absent (the table holds {} slots)", bank.records.len())))?;
    let (ok, refusal) = preview_gate(game_id, bank, rec, platform);
    if !ok {
        return Err(DomainError::Validation(format!("{bank_name} record {record}: {}", refusal.unwrap_or_default())));
    }
    if bank.platform == BankPlatform::Xbox360 {
        let mut p = if matches!(rec.body, RecordBody::Stream { .. }) {
            xma_stream_preview(game_id, platform, bank_name, record, &sources, &l, rec, sub_stream)?
        } else {
            xma_preview(game_id, platform, bank_name, record, &l, rec, sub_stream)?
        };
        if platform == crate::Platform::Recomp {
            p.notes.push(RECOMP_PREVIEW_NOTE.into());
        }
        return Ok(p);
    }
    let n = abk::sub_streams(rec.header.channel_config).unwrap_or(1);
    let part = match (n, sub_stream) {
        (1, None | Some(0)) => None,
        (1, Some(k)) => return Err(DomainError::Validation(format!("{bank_name} record {record}: one stream only; sub-stream {} does not exist", k + 1))),
        (n, None) => return Err(DomainError::Validation(format!("{bank_name} record {record}: channel config {} carries {n} interleaved stereo streams — choose one (1–{n})", rec.header.channel_config))),
        (n, Some(k)) if k as usize >= n => return Err(DomainError::Validation(format!("{bank_name} record {record}: sub-stream {} of {n} does not exist", k + 1))),
        (_, Some(k)) => Some(k as usize),
    };
    let owners: Vec<String> = bank.lists_naming(record).iter().flat_map(|l| l.owners.iter().cloned()).collect();
    let mut stub: Option<(u32, Vec<u8>)> = None;
    let (source, source_bytes, blocks): (String, Vec<u8>, Vec<(u32, Vec<u8>)>) = match &rec.body {
        RecordBody::Ram { segments, .. } => {
            // Ram: the segments are the blocks (R82: decode on ram segments under R77).
            let all: Vec<u8> = segments.iter().flat_map(|s| s.bytes.iter().copied()).collect();
            (
                format!("{} record {record} (ram, {} segment(s) inline)", l.loc.describe(), segments.len()),
                all,
                segments.iter().map(|s| (s.samples, s.bytes.clone())).collect(),
            )
        }
        RecordBody::Stream { loop_words } => {
            let entry = bank
                .lists
                .iter()
                .flat_map(|l| l.entries.iter())
                .find(|e| e.index == record)
                .ok_or_else(|| DomainError::Validation(format!("record {record}: no list entry names it")))?;
            let off = entry.stream_offset.ok_or_else(|| DomainError::Validation(format!("record {record}: stream record without a stream offset")))? as u64;
            let loc = stream_for(&sources, bank_name).map_err(DomainError::Validation)?;
            let Framed { main: s, body } = frame_stream(&loc, bank.ast_id, off, rec)?;
            // Loops: the referenced stream is the stub; the blocks decoded are the body's.
            let (data, what) = match (loop_words, body) {
                (Some((a, _)), Some(bd)) => {
                    stub = Some((*a, s.blocks[0].payload.clone()));
                    (bd, format!("loop stub at {off} + body at {s_end}", s_end = s.end))
                }
                _ => (s, format!("stream at {off}")),
            };
            let raw: Vec<u8> = stub.iter().flat_map(|(_, p)| p.iter().copied()).chain(data.blocks.iter().flat_map(|b| b.payload.iter().copied())).collect();
            (
                format!("{} {what} ({} blocks, {} B)", loc.describe(), data.blocks.len(), data.end - data.start),
                raw,
                data.blocks.iter().map(|b| (b.samples, b.payload.clone())).collect(),
            )
        }
    };
    let err = |e: String| DomainError::Validation(format!("{bank_name} record {record}: {e}"));
    let refs: Vec<(u32, &[u8])> = blocks.iter().map(|(n, p)| (*n, p.as_slice())).collect();
    let split;
    let refs: Vec<(u32, &[u8])> = match part {
        None => refs,
        Some(k) => {
            split = abk::split_sub_streams(&refs, n).map_err(err)?;
            split[k].iter().map(|(n, p)| (*n, p.as_slice())).collect()
        }
    };
    let d = match &stub {
        None => abk::eal3_diagnostic(&refs, rec.header.num_samples).map_err(err)?,
        Some((a, stub_bytes)) => {
            let stub_part = match part {
                None => stub_bytes.clone(),
                Some(k) => abk::split_sub_streams(&[(*a, stub_bytes.as_slice())], n).map_err(err)?.swap_remove(k).swap_remove(0).1,
            };
            abk::eal3_loop_diagnostic(&stub_part, *a, &refs, rec.header.num_samples - a).map_err(err)?
        }
    };
    let multi = part.is_some() || stub.is_some();
    Ok(Gen7SfxPreview {
        game_id: game_id.to_string(),
        platform: platform_label(platform).to_string(),
        bank: bank_name.to_string(),
        bank_sha256: format!("{:x}", Sha256::digest(&l.bytes)),
        record,
        kind: rec.kind().to_string(),
        sub_stream: part.map(|k| k as u16),
        sub_streams: n as u16,
        loop_start: stub.as_ref().map(|(a, _)| *a),
        owners,
        source,
        source_sha256: format!("{:x}", Sha256::digest(&source_bytes)),
        codec: rec.header.codec_label().to_string(),
        sample_rate: d.sample_rate,
        channels: d.channels,
        num_samples: rec.header.num_samples,
        blocks: blocks.len(),
        granules: d.granules,
        rebuilt_frames: d.rebuilt_frames,
        decoded_frames_per_channel: d.decoded_frames_per_channel,
        label: if multi { abk::EAL3_PART_LABEL } else { abk::EAL3_AST_LABEL }.to_string(),
        wav: tdb_snd::wav::encode_pcm16(d.sample_rate, d.channels as u16, &d.pcm),
        notes: vec![
            format!("The granules were rebuilt into {} standard MPEG frames and decoded by Symphonia; the WAV is the R92 candidate construction: the record's 47-sample PCM frame, then the decoder's output from sample 1 152, trimmed to the declared {} samples. Which of the two copies of those first samples the console plays is not established (card A-AST-1).", d.rebuilt_frames, rec.header.num_samples),
            layout.evidence.into(),
            "Diagnostic only: not game-verified, not listened to by the developer; replacement is not supported.".into(),
        ],
    })
}
