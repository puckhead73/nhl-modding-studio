//! Phase 1.5 / 2.7 / 3.5 — the **read-only gen7 commentary catalogue** (PS3 / Xbox 360 NHL 11–Legacy):
//! the speech events with their parameters, the sentences that carry an event's name with their
//! bodies and phrases, each phrase resolved through the graffiti table to its candidate samples
//! and to the bank, and the keyword (mention) index for the team / arena `audioid` joins.
//! Consumes research 2026-09-21 handoff 03 rev. 2a (the read-only contract) and handoff 04 (the
//! speech model), parsed by [`tdb_speech`].
//!
//! What it says and does not say (the contract's wording): a sentence is "a sentence sharing this
//! event's name", never "the line this event speaks"; an event without one is "no sentence carries
//! this event name" (never "silent"); a phrase lists *candidate* samples — "the event's
//! <parameters> select among them" when it carries event-parameter tags; the runtime (which
//! context fires, which take plays) is unobserved; nothing here previews or plays a sample (no
//! decoder) and nothing writes.
//!
//! Sources (handoff 02 §2 of 2026-09-21): the loose `audio/speech` files where they exist (equal
//! to the `audioboot.big` members on every measured tree). The archive-only trees (PS3 NHL 12 /
//! 13) and the NHL 09 / 10 XML form are **not read tonight** — the catalogue says so instead of
//! reading nothing.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{DomainError, Result};

/// Where a catalogue input was read and its identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechSourceFile {
    pub role: String,
    pub path: String,
    pub len: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechEvent {
    pub id: u16,
    pub name: String,
    /// The parameter names, in order (`?<id>` when the id is not in the parameter table).
    pub parameters: Vec<String>,
    /// Other names the same 16-bit id carries in the `MOIR` table (all candidates kept; nothing chosen).
    pub other_names_for_id: Vec<String>,
    /// How many sentences carry exactly this name (0 = "no sentence carries this event name").
    pub sentences: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechPhrase {
    pub pool_id: u32,
    /// The fixed `SecretName_SampleBank` tag id and value (the sample group).
    pub group_tag: u32,
    pub group_value: u32,
    /// The group's name where the file carries one (named NHL 11 graffiti entries); else `None`.
    pub group_name: Option<String>,
    /// The event's parameter names the phrase's event-reference tags take their values from.
    pub parameters: Vec<String>,
    /// Whether the graffiti table `(tag, value, numParameters)` exists.
    pub table_found: bool,
    /// Candidate sample ids of that table (all of them; the parameters select among them at runtime).
    pub candidate_sample_ids: Vec<u32>,
    /// How many of the candidates resolve to a bank entry (a `.sbs` offset).
    pub bank_resolved: usize,
    /// Candidate ids absent from the bank (reported, never dropped).
    pub unresolved_sample_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechBody {
    pub id: u32,
    pub phrases: Vec<SpeechPhrase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechSentence {
    pub name: String,
    /// The event of the same name, when one exists.
    pub event_id: Option<u16>,
    pub slot: u32,
    pub slot_ok: bool,
    pub bodies: Vec<SpeechBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SpeechStats {
    pub events: usize,
    pub events_with_a_sentence: usize,
    pub events_without_a_sentence: usize,
    pub parameters: usize,
    pub sentences: usize,
    pub sentences_slot_ok: usize,
    pub bodies: usize,
    pub phrases: usize,
    pub phrases_with_table: usize,
    pub graffiti_tables: usize,
    pub bank_samples: usize,
    pub keywords: usize,
    pub ambiguous_event_ids: usize,
}

/// The whole catalogue of one title's speech data (read-only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechCatalogue {
    pub game_id: String,
    pub platform: String,
    pub speech_dir: String,
    pub sources: Vec<SpeechSourceFile>,
    pub events: Vec<SpeechEvent>,
    pub sentences: Vec<SpeechSentence>,
    /// `MOIR` ids that carry several names, with every name.
    pub ambiguous_ids: BTreeMap<u16, Vec<String>>,
    /// Keyword id → sample ids (the mention index; the team / arena / city / chant `audioid`
    /// columns are keyword ids of this table). Empty when the title ships no keyword file.
    pub keyword_sample_counts: BTreeMap<u32, usize>,
    pub stats: SpeechStats,
    pub notes: Vec<String>,
}

fn sha_hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// The regex-free "league / competition" name test of the research trace (AHL, CHL, LEAGUE, CUP,
/// SPENGLER, MEMORIAL, CALDER, KELLY, INTERNATIONAL, TOURNAMENT).
pub fn is_league_name(name: &str) -> bool {
    const KEYS: &[&str] = &["AHL", "CHL", "LEAGUE", "CUP", "SPENGLER", "MEMORIAL", "CALDER", "KELLY", "INTERNATIONAL", "TOURNAMENT"];
    let up = name.to_ascii_uppercase();
    KEYS.iter().any(|k| up.contains(k))
}

/// Whether this title / platform is inside the model's measured coverage for the loose route.
fn coverage(game_id: &str, platform: crate::Platform) -> std::result::Result<(), String> {
    match (game_id, platform) {
        ("nhl11" | "nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy" | "nhllegacy-recompiled" | "legacyrecomp", crate::Platform::Xbox | crate::Platform::Recomp) => Ok(()),
        ("nhl11" | "nhl14" | "nhl15" | "nhllegacy", crate::Platform::Ps3) => Ok(()),
        ("nhl12" | "nhl13", crate::Platform::Ps3) => Err(format!("{game_id} PS3 ships its speech tables only inside audioboot.big (chunkzip members); the archive route of the catalogue is not built yet — nothing is listed rather than something guessed")),
        ("nhl09" | "nhl10", _) => Err(format!("{game_id} ships the speech tables in EA's XML source form; the XML reader is not built yet")),
        ("nhl07" | "nhl08", _) => Err(format!("{game_id} uses the older pbp_eng.{{big,csi,cti,evt,idx,nam}} speech system, which the research model does not cover")),
        (_, crate::Platform::Ps4) => Err("PS4 titles use the SBle / pbp_sentencedata chain (research 2026-09-20 handoff 02); the PS4 catalogue is a separate package".into()),
        _ => Err(format!("{game_id} on this platform is outside the measured speech-model coverage")),
    }
}

/// Where one title's `audio/speech` or `audio/context` tables are read (receipt 67; research 2026-09-28 handoff 16): the loose
/// directory, or — Legacy Recompiled only, when that directory is absent — the install's own archive members.
pub(crate) enum AudioDir {
    Loose(PathBuf),
    /// Archive members `<rel>/<file>`, searched in `archives` order (`audioboot.big`, `nocache.big`, then the rest).
    Members { rel: &'static str, archives: Vec<(PathBuf, String)> },
}

fn recompiled(game_id: &str, platform: crate::Platform) -> bool {
    platform == crate::Platform::Recomp && matches!(game_id, "legacyrecomp" | "nhllegacy-recompiled")
}

impl AudioDir {
    pub(crate) fn open(game_id: &str, platform: crate::Platform, install_root: &Path, rel: &'static str) -> Result<Self> {
        let dir: PathBuf = crate::assets::resolve_install_path(install_root, rel).ok_or_else(|| DomainError::Validation(format!("{rel} escapes the install root")))?;
        if dir.is_dir() {
            return Ok(AudioDir::Loose(dir));
        }
        if !recompiled(game_id, platform) {
            return Err(DomainError::Validation(format!("{}: no loose {rel} directory under this root (the archive route is built for Legacy Recompiled only)", dir.display())));
        }
        // The install folder (`game/*.big`) or its `game` folder (`*.big`): the named archives first, then every other one.
        let mut archives: Vec<(PathBuf, String)> = ["audioboot.big", "nocache.big"].iter().filter_map(|n| crate::gen7_speech_preview::archive_beside(install_root, n)).collect();
        for base in [install_root.join("game"), install_root.to_path_buf()] {
            let prefix = if base == install_root { "" } else { "game/" };
            let mut rest: Vec<(PathBuf, String)> = std::fs::read_dir(&base)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("big")))
                .map(|p| {
                    let label = format!("{prefix}{}", p.file_name().unwrap().to_string_lossy());
                    (p, label)
                })
                .filter(|(p, _)| !archives.iter().any(|(q, _)| q == p))
                .collect();
            rest.sort_by(|a, b| a.1.cmp(&b.1));
            archives.extend(rest);
        }
        if archives.is_empty() {
            return Err(DomainError::Validation(format!(
                "{}: no loose {rel} directory, and no archives in this folder or its `game` folder to read the members from",
                dir.display()
            )));
        }
        Ok(AudioDir::Members { rel, archives })
    }

    /// Read one file; `Ok(None)` only when it is absent and not `required`. Returns the bytes and a provenance line.
    pub(crate) fn read(&self, name: &str, required: bool) -> Result<Option<(Vec<u8>, String)>> {
        match self {
            AudioDir::Loose(dir) => {
                let p = dir.join(name);
                match std::fs::read(&p) {
                    Ok(b) => Ok(Some((b, p.display().to_string()))),
                    Err(_) if !required => Ok(None),
                    Err(e) => Err(DomainError::Validation(format!("{}: {e}", p.display()))),
                }
            }
            AudioDir::Members { rel, archives } => {
                let member = format!("{rel}/{name}");
                let mut hits: Vec<(Vec<u8>, String)> = Vec::new();
                for (archive, label) in archives {
                    if let Some((bytes, form, _, size)) = crate::gen7_speech_preview::read_member_unwrapped(archive, &member).map_err(DomainError::Validation)? {
                        hits.push((bytes, format!("{label}:{member} ({form}, {size} B stored)")));
                    }
                }
                match hits.len() {
                    0 if !required => Ok(None),
                    0 => Err(DomainError::Validation(format!(
                        "no `{member}` member in {} (and no loose {rel} directory)",
                        archives.iter().map(|(_, l)| l.as_str()).collect::<Vec<_>>().join(", ")
                    ))),
                    1 => Ok(hits.pop()),
                    _ if hits.iter().all(|(b, _)| *b == hits[0].0) => {
                        let also: Vec<String> = hits[1..].iter().map(|(_, d)| d.clone()).collect();
                        let (b, d) = hits.swap_remove(0);
                        Ok(Some((b, format!("{d}; identical copy in {}", also.join(", ")))))
                    }
                    _ => Err(DomainError::Validation(format!(
                        "`{member}` is in {} archives with different bytes ({}); Studio does not choose between them",
                        hits.len(),
                        hits.iter().map(|(_, d)| d.as_str()).collect::<Vec<_>>().join("; ")
                    ))),
                }
            }
        }
    }

    pub(crate) fn describe(&self) -> String {
        match self {
            AudioDir::Loose(dir) => dir.display().to_string(),
            AudioDir::Members { rel, archives } => format!("{rel} archive members ({})", archives.iter().map(|(_, l)| l.as_str()).take(3).collect::<Vec<_>>().join(", ")),
        }
    }
}

/// The provenance note of an archive-member read (research R128 / R145).
const MEMBER_NOTE: &str = "read from this Legacy Recompiled install's own archive members (no loose audio tree): their unwrapped bytes are identical to the Xbox 360 Legacy files (research R128 / R145); what the recompiled game plays is not observed";

/// Build the catalogue from the `audio/speech` files under `install_root` (loose; on Legacy Recompiled, archive members).
pub fn speech_catalogue(game_id: &str, platform: crate::Platform, install_root: &Path) -> Result<SpeechCatalogue> {
    coverage(game_id, platform).map_err(DomainError::Validation)?;
    let src = AudioDir::open(game_id, platform, install_root, "audio/speech")?;
    let dir = PathBuf::from(src.describe());
    let mut sources = Vec::new();
    let mut read = |role: &str, name: &str, required: bool| -> Result<Option<Vec<u8>>> {
        Ok(src.read(name, required)?.map(|(b, path)| {
            sources.push(SpeechSourceFile { role: role.into(), path, len: b.len() as u64, sha256: sha_hex(&b) });
            b
        }))
    };
    let evt_b = read("events", "pbp_eng_speecheventsubscr.bin", true)?.unwrap();
    let moir_b = read("names", "pbp_eng_speecheventdef.csi", true)?.unwrap();
    let sent_b = read("sentences", "pbp_eng_sentencedata.bin", true)?.unwrap();
    let grf_b = read("graffiti", "pbp_eng_graffitidata.bin", true)?.unwrap();
    let bank_b = read("bank", "pbp_eng_samples.sbr", true)?.unwrap();
    let kw_b = read("keywords", "pbp_eng_keywords.bin", false)?;
    let bad = |what: &str, e: tdb_speech::SpeechError| DomainError::Validation(format!("{}: {what}: {e}", dir.display()));
    let evt = tdb_speech::parse_evt0(&evt_b).map_err(|e| bad("events", e))?;
    let moir = tdb_speech::parse_moir(&moir_b).map_err(|e| bad("names", e))?;
    let sent = tdb_speech::parse_sentences(&sent_b).map_err(|e| bad("sentences", e))?;
    let grf = tdb_speech::parse_graffiti(&grf_b).map_err(|e| bad("graffiti", e))?;
    let bank = tdb_speech::parse_bank(&bank_b).map_err(|e| bad("bank", e))?;
    let keywords = match kw_b {
        Some(b) => Some(tdb_speech::parse_keywords(&b[..]).map_err(|e| bad("keywords", e))?),
        None => None,
    };
    let group_names = grf.tag_names();
    let mut notes = vec![
        "read-only: sentences are listed as sharing an event's name — which line the engine speaks, which context fires and which take plays are runtime questions nobody has observed; no sample is decoded or played here".to_string(),
        "a phrase's samples are the candidates of its sample-group table; where the phrase carries event-parameter tags, the event's parameters select among them at runtime".to_string(),
    ];
    if matches!(src, AudioDir::Members { .. }) {
        notes.push(MEMBER_NOTE.into());
    }
    if keywords.is_none() {
        notes.push("no pbp_eng_keywords.bin is loose on this title (NHL 11–14): the keyword (mention) index that binds the team / arena audioid columns is not available here".into());
    }
    if evt.unclassified > 0 {
        notes.push(format!("{} records of the event file are neither parameters nor events of the measured shape (skipped, listed here)", evt.unclassified));
    }
    if grf.malformed > 0 {
        notes.push(format!("{} graffiti entries had pointers past the file (skipped, listed here)", grf.malformed));
    }
    let param_name = |id: u32| evt.param_name(id).map(str::to_string).unwrap_or_else(|| format!("?{id}"));
    let sentence_counts: HashMap<&str, usize> = sent.sentences.iter().fold(HashMap::new(), |mut m, s| {
        *m.entry(s.name.as_str()).or_default() += 1;
        m
    });
    let ambiguous_ids = moir.ambiguous_ids();
    let mut events: Vec<SpeechEvent> = evt
        .events
        .iter()
        .map(|e| SpeechEvent {
            id: e.id,
            name: e.name.clone(),
            parameters: e.params.iter().map(|p| param_name(*p)).collect(),
            other_names_for_id: moir.names(e.id).into_iter().filter(|n| *n != e.name).map(str::to_string).collect(),
            sentences: sentence_counts.get(e.name.as_str()).copied().unwrap_or(0),
        })
        .collect();
    events.sort_by(|a, b| a.name.cmp(&b.name));
    let ev_by_name: HashMap<&str, &tdb_speech::Event> = evt.events.iter().map(|e| (e.name.as_str(), e)).collect();
    let mut stats = SpeechStats { events: events.len(), parameters: evt.params.len(), graffiti_tables: grf.tables.len(), bank_samples: bank.samples.len(), keywords: keywords.as_ref().map(|k| k.by_keyword.len()).unwrap_or(0), ambiguous_event_ids: ambiguous_ids.len(), ..Default::default() };
    stats.events_with_a_sentence = events.iter().filter(|e| e.sentences > 0).count();
    stats.events_without_a_sentence = events.len() - stats.events_with_a_sentence;
    let mut sentences = Vec::with_capacity(sent.sentences.len());
    for s in &sent.sentences {
        let ev = ev_by_name.get(s.name.as_str());
        let mut bodies = Vec::with_capacity(s.bodies.len());
        for b in &s.bodies {
            let mut phrases = Vec::with_capacity(b.phrases.len());
            for p in &b.phrases {
                let (mut group_tag, mut group_value) = (0u32, 0u32);
                let mut parameters = Vec::new();
                for t in &p.tags {
                    match t {
                        tdb_speech::PhraseTag::Fixed { tag, value } => {
                            group_tag = *tag;
                            group_value = *value;
                        }
                        tdb_speech::PhraseTag::EventRef { tag, event_index } => {
                            let name = match ev.and_then(|e| e.params.get(*event_index as usize)) {
                                Some(id) if *id == *tag => param_name(*id),
                                Some(id) => format!("{} (parameter {event_index} of the event; tag {})", param_name(*id), param_name(*tag)),
                                None => format!("{} (index {event_index} beyond the event's parameters)", param_name(*tag)),
                            };
                            parameters.push(name);
                        }
                    }
                }
                let table = grf.table(group_tag, group_value, p.num_parameters);
                let candidate_sample_ids: Vec<u32> = table.map(|t| t.sample_ids.clone()).unwrap_or_default();
                let unresolved: Vec<u32> = candidate_sample_ids.iter().copied().filter(|id| bank.sample(*id).is_none()).collect();
                if table.is_some() {
                    stats.phrases_with_table += 1;
                }
                stats.phrases += 1;
                phrases.push(SpeechPhrase {
                    pool_id: p.pool_id,
                    group_tag,
                    group_value,
                    group_name: table.and_then(|t| t.name.clone()).or_else(|| group_names.get(&group_tag).map(|n| format!("{n}_{group_value}"))),
                    parameters,
                    table_found: table.is_some(),
                    bank_resolved: candidate_sample_ids.len() - unresolved.len(),
                    unresolved_sample_ids: unresolved,
                    candidate_sample_ids,
                });
            }
            stats.bodies += 1;
            bodies.push(SpeechBody { id: b.id, phrases });
        }
        if s.slot_ok {
            stats.sentences_slot_ok += 1;
        }
        sentences.push(SpeechSentence { name: s.name.clone(), event_id: ev.map(|e| e.id), slot: s.slot, slot_ok: s.slot_ok, bodies });
    }
    stats.sentences = sentences.len();
    sentences.sort_by(|a, b| a.name.cmp(&b.name).then(a.slot.cmp(&b.slot)));
    let bank_ids: HashSet<u32> = bank.samples.iter().map(|s| s.id).collect();
    let keyword_sample_counts = keywords.as_ref().map(|k| k.by_keyword.iter().map(|(kw, ids)| (*kw, ids.iter().filter(|i| bank_ids.contains(i)).count())).collect()).unwrap_or_default();
    Ok(SpeechCatalogue {
        game_id: game_id.to_string(),
        platform: platform.as_str().to_string(),
        speech_dir: dir.display().to_string(),
        sources,
        events,
        sentences,
        ambiguous_ids,
        keyword_sample_counts,
        stats,
        notes,
    })
}

/// A bounded view of the catalogue for a UI page: events and sentences whose name contains
/// `query` (case-insensitive; empty = all), optionally league / competition names only, capped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechCatalogueView {
    pub game_id: String,
    pub platform: String,
    pub speech_dir: String,
    pub sources: Vec<SpeechSourceFile>,
    pub stats: SpeechStats,
    pub notes: Vec<String>,
    pub query: String,
    pub league_only: bool,
    pub events_matching: usize,
    pub sentences_matching: usize,
    pub events: Vec<SpeechEvent>,
    pub sentences: Vec<SpeechSentence>,
    pub truncated: bool,
}

pub fn view(c: &SpeechCatalogue, query: &str, league_only: bool, limit: usize) -> SpeechCatalogueView {
    let q = query.trim().to_ascii_lowercase();
    let keep = |name: &str| (q.is_empty() || name.to_ascii_lowercase().contains(&q)) && (!league_only || is_league_name(name));
    let events: Vec<&SpeechEvent> = c.events.iter().filter(|e| keep(&e.name)).collect();
    let sentences: Vec<&SpeechSentence> = c.sentences.iter().filter(|s| keep(&s.name)).collect();
    let truncated = events.len() > limit || sentences.len() > limit;
    SpeechCatalogueView {
        game_id: c.game_id.clone(),
        platform: c.platform.clone(),
        speech_dir: c.speech_dir.clone(),
        sources: c.sources.clone(),
        stats: c.stats.clone(),
        notes: c.notes.clone(),
        query: query.to_string(),
        league_only,
        events_matching: events.len(),
        sentences_matching: sentences.len(),
        events: events.into_iter().take(limit).cloned().collect(),
        sentences: sentences.into_iter().take(limit).cloned().collect(),
        truncated,
    }
}

/// Phase 2.7 (21 Sep 2026) — the name-call listing of one team (read-only): the recorded samples
/// tagged with this team's `exhibitionteams.audioid` in the play-by-play graffiti tables keyed by
/// the `team_ID` parameter (research 2026-09-21 handoff 07 §1–2: 207 / 211 team audio ids are
/// `team_ID` keys on NHL 15). One entry per tag count (`num_tags` = how many tags the samples carry
/// besides the team: 1 = the team alone, 2 = team + one more parameter, …).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeamNameCalls {
    pub team_audio_id: u64,
    /// The `team_ID` parameter id of this title's event table.
    pub team_tag: u32,
    pub tables: Vec<NameCallTable>,
    pub sample_ids_distinct: usize,
    pub bank_resolved_distinct: usize,
    /// The keyword (mention) index entries for this id, when the title ships the index
    /// (NHL 15 / Legacy): samples that mention the club, a separate space from the name calls.
    pub keyword_mentions: Option<usize>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NameCallTable {
    pub num_tags: u32,
    pub sample_ids: Vec<u32>,
    pub bank_resolved: usize,
}

/// List the play-by-play samples tagged with `team_audio_id` (the team's `audioid` column).
pub fn team_name_calls(game_id: &str, platform: crate::Platform, install_root: &Path, team_audio_id: u64) -> Result<TeamNameCalls> {
    coverage(game_id, platform).map_err(DomainError::Validation)?;
    let src = AudioDir::open(game_id, platform, install_root, "audio/speech")?;
    let read = |name: &str| src.read(name, true).map(|o| o.expect("required").0);
    let bad = |what: &str, e: tdb_speech::SpeechError| DomainError::Validation(format!("{}: {what}: {e}", src.describe()));
    let evt = tdb_speech::parse_evt0(&read("pbp_eng_speecheventsubscr.bin")?).map_err(|e| bad("events", e))?;
    let grf = tdb_speech::parse_graffiti(&read("pbp_eng_graffitidata.bin")?).map_err(|e| bad("graffiti", e))?;
    let bank = tdb_speech::parse_bank(&read("pbp_eng_samples.sbr")?).map_err(|e| bad("bank", e))?;
    let team_tag = evt.params.iter().find(|p| p.name == "team_ID").map(|p| p.id).ok_or_else(|| DomainError::Validation("this title's speech event table has no `team_ID` parameter; the name-call join of research 2026-09-21 handoff 07 does not apply".into()))?;
    let keywords = src.read("pbp_eng_keywords.bin", false)?.and_then(|(b, _)| tdb_speech::parse_keywords(&b).ok());
    let value = u32::try_from(team_audio_id).map_err(|_| DomainError::Validation(format!("audio id {team_audio_id} is outside the 32-bit tag value space")))?;
    // Audio id 0 is the "no recorded name" value (the research's binding counts are over the
    // non-zero ids); a (team_ID, 0) table, when one exists, is not this team's name.
    let mut tables: Vec<NameCallTable> = grf
        .tables
        .iter()
        .filter(|t| value != 0 && t.tag == team_tag && t.value == value)
        .map(|t| NameCallTable { num_tags: t.num_tags, bank_resolved: t.sample_ids.iter().filter(|id| bank.sample(**id).is_some()).count(), sample_ids: t.sample_ids.clone() })
        .collect();
    tables.sort_by_key(|t| t.num_tags);
    let distinct: HashSet<u32> = tables.iter().flat_map(|t| t.sample_ids.iter().copied()).collect();
    let resolved = distinct.iter().filter(|id| bank.sample(**id).is_some()).count();
    let mut notes = vec!["read-only: the samples tagged with this team's audio id in the play-by-play graffiti tables (the recorded name calls and club-mentioning lines); which line plays, and when, is runtime — nothing is decoded or played here".to_string()];
    if value == 0 {
        notes.push("audio id 0: this team has no recorded name id (the play-by-play never names it)".into());
    } else if tables.is_empty() {
        notes.push(format!("no play-by-play table is keyed by audio id {team_audio_id} — the game has no recorded call for this id (the research lists 4 / 211 such club ids on NHL 15); a reassignment to another club's recorded id would be a database edit validated against this key set (contract proposed, not implemented)"));
    }
    if keywords.is_none() {
        notes.push("no keyword (mention) index is loose on this title (NHL 11–14)".into());
    }
    Ok(TeamNameCalls {
        team_audio_id,
        team_tag,
        keyword_mentions: keywords.as_ref().map(|k| k.by_keyword.get(&value).map(|v| v.len()).unwrap_or(0)),
        tables,
        sample_ids_distinct: distinct.len(),
        bank_resolved_distinct: resolved,
        notes,
    })
}

/// The team's `exhibitionteams.audioid` (its play-by-play name id) and name, read from the session.
pub fn team_audio_id(session: &crate::session::EditSession, team_id: u64) -> Result<(String, Option<u64>)> {
    let (record, name) = crate::team_reid::team_row(session, team_id)?;
    let bytes = session.bytes().to_vec();
    let db = tdb_core::TdbFile::parse(&bytes)?;
    let meta = session.project().meta();
    let kott = db.table_canonical(tdb_relations::entity::ids::TEAM).ok_or_else(|| DomainError::Validation("no team table".into()))?;
    let id = crate::view::descriptor_by_name(meta, kott, "audioid").and_then(|f| db.read_u64(kott, record, f));
    Ok((name, id))
}

// ------------------------------------------------------------------ Phase 3.5: player name calls

/// Phase 3.5 (21 Sep 2026) — one player's recorded names, read-only (research 2026-09-21 handoff
/// 07 §1–2): the play-by-play side joins the `playernames` catalogue rows matching the player's
/// first / last name text to the pbp `player_ID` graffiti tables; the rink-announcer side joins
/// `exhibitionplayerbiotable.audioid` to the announcer module's `playerid` tables (id 2163 on
/// every gen7 title — named by NHL 09's XML, unnamed in the later binaries).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerNameCalls {
    pub player_id: u64,
    pub first_name: String,
    pub last_name: String,
    /// `playernames` rows whose text equals the player's first name (`isfirstname = 1`) or last
    /// name (`isfirstname = 0`), case-insensitively, each with its pbp `player_ID` tables.
    pub pbp_names: Vec<PbpNameEntry>,
    pub pbp_player_tag: Option<u32>,
    /// `exhibitionplayerbiotable.audioid`.
    pub announcer_audio_id: Option<u64>,
    pub announcer_tag: Option<u32>,
    pub announcer_tables: Vec<NameCallTable>,
    pub announcer_sample_ids_distinct: usize,
    pub announcer_bank_resolved_distinct: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PbpNameEntry {
    pub name: String,
    pub is_first_name: bool,
    pub bank: u64,
    pub audio_id: u64,
    pub tables: Vec<NameCallTable>,
    pub sample_ids_distinct: usize,
    pub bank_resolved_distinct: usize,
}

/// The announcer module's `playerid` parameter (research 2026-09-21 handoff 07 §2: named by
/// NHL 09's `ann_eng_speecheventsubscr.xml`; unnamed "tag 2163" on the later binaries).
pub const ANNOUNCER_PLAYER_TAG: u32 = 2163;

fn name_call_tables(grf: &tdb_speech::Graffiti, bank: &tdb_speech::Bank, tag: u32, value: u32) -> (Vec<NameCallTable>, usize, usize) {
    let mut tables: Vec<NameCallTable> = grf
        .tables
        .iter()
        .filter(|t| value != 0 && t.tag == tag && t.value == value)
        .map(|t| NameCallTable { num_tags: t.num_tags, bank_resolved: t.sample_ids.iter().filter(|id| bank.sample(**id).is_some()).count(), sample_ids: t.sample_ids.clone() })
        .collect();
    tables.sort_by_key(|t| t.num_tags);
    let distinct: HashSet<u32> = tables.iter().flat_map(|t| t.sample_ids.iter().copied()).collect();
    let resolved = distinct.iter().filter(|id| bank.sample(**id).is_some()).count();
    (tables, distinct.len(), resolved)
}

/// The play-by-play and announcer recordings of one roster player (read-only).
pub fn player_name_calls(session: &crate::session::EditSession, game_id: &str, platform: crate::Platform, install_root: &Path, player_id: u64) -> Result<PlayerNameCalls> {
    coverage(game_id, platform).map_err(DomainError::Validation)?;
    let bytes = session.bytes().to_vec();
    let db = tdb_core::TdbFile::parse(&bytes)?;
    let meta = session.project().meta();
    let model = session.model()?;
    let p = model.player(player_id).ok_or_else(|| DomainError::Validation(format!("no player with id {player_id}")))?;
    let bio = db.table_canonical(tdb_relations::entity::ids::BIO).ok_or_else(|| DomainError::Validation("no player bio table".into()))?;
    let first_name = crate::view::read_str_by_name(&db, meta, bio, p.bio_record, "firstname").trim().to_string();
    let last_name = crate::view::read_str_by_name(&db, meta, bio, p.bio_record, "lastname").trim().to_string();
    let announcer_audio_id = crate::view::descriptor_by_name(meta, bio, "audioid").and_then(|f| db.read_u64(bio, p.bio_record, f));

    let src = AudioDir::open(game_id, platform, install_root, "audio/speech")?;
    let read = |name: &str| src.read(name, true).map(|o| o.expect("required").0);
    let bad = |what: &str, e: tdb_speech::SpeechError| DomainError::Validation(format!("{}: {what}: {e}", src.describe()));
    let evt = tdb_speech::parse_evt0(&read("pbp_eng_speecheventsubscr.bin")?).map_err(|e| bad("pbp events", e))?;
    let grf = tdb_speech::parse_graffiti(&read("pbp_eng_graffitidata.bin")?).map_err(|e| bad("pbp graffiti", e))?;
    let bank = tdb_speech::parse_bank(&read("pbp_eng_samples.sbr")?).map_err(|e| bad("pbp bank", e))?;
    let pbp_player_tag = evt.params.iter().find(|q| q.name == "player_ID").map(|q| q.id);
    let mut notes = vec!["read-only: the recorded samples the play-by-play tags with this player's name rows and the announcer tags with the player's audio id; which line plays is runtime — nothing is decoded, played or written".to_string()];

    // Play-by-play: the `playernames` catalogue rows matching the name text.
    let mut pbp_names = Vec::new();
    match (crate::team_reid::table_by_name(&db, meta, "playernames"), pbp_player_tag) {
        (Some(t), Some(tag)) => {
            let (Some(name_f), Some(first_f), Some(bank_f), Some(audio_f)) = (
                crate::view::descriptor_by_name(meta, t, "name"),
                crate::view::descriptor_by_name(meta, t, "isfirstname"),
                crate::view::descriptor_by_name(meta, t, "bank"),
                crate::view::descriptor_by_name(meta, t, "audioid"),
            ) else {
                return Err(DomainError::Validation("the playernames table lacks the name / isfirstname / bank / audioid columns".into()));
            };
            for r in 0..t.current_records as u32 {
                let name = db.read_inline_str(t, r, name_f).map(|b| String::from_utf8_lossy(b).trim().to_string()).unwrap_or_default();
                let is_first = db.read_u64(t, r, first_f) == Some(1);
                let want = if is_first { &first_name } else { &last_name };
                if name.is_empty() || want.is_empty() || !name.eq_ignore_ascii_case(want) {
                    continue;
                }
                let audio_id = db.read_u64(t, r, audio_f).unwrap_or(0);
                let value = u32::try_from(audio_id).unwrap_or(0);
                let (tables, sample_ids_distinct, bank_resolved_distinct) = name_call_tables(&grf, &bank, tag, value);
                pbp_names.push(PbpNameEntry { name, is_first_name: is_first, bank: db.read_u64(t, r, bank_f).unwrap_or(0), audio_id, tables, sample_ids_distinct, bank_resolved_distinct });
            }
            if pbp_names.is_empty() {
                notes.push(format!("no playernames row spells \"{first_name}\" or \"{last_name}\": the play-by-play has no recorded name for this player (the catalogue is matched by name text)"));
            } else if pbp_names.iter().any(|n| n.tables.is_empty()) {
                notes.push("a name is in the catalogue but no play-by-play table is keyed by its audio id: listed, not offered (handoff 07 §2 row 1)".into());
            }
        }
        (None, _) => notes.push("this title's database has no playernames catalogue; the play-by-play join does not apply".into()),
        (_, None) => notes.push("the play-by-play event table has no `player_ID` parameter; the name join does not apply".into()),
    }

    // Announcer: the bio audio id under the announcer module's playerid tag.
    let (announcer_tag, announcer_tables, ann_distinct, ann_resolved) = match (src.read("ann_eng_graffitidata.bin", false)?, src.read("ann_eng_samples.sbr", false)?) {
        (Some((g, _)), Some((s, _))) => {
            let ann_grf = tdb_speech::parse_graffiti(&g).map_err(|e| bad("announcer graffiti", e))?;
            let ann_bank = tdb_speech::parse_bank(&s).map_err(|e| bad("announcer bank", e))?;
            let tag = src
                .read("ann_eng_speecheventsubscr.bin", false)?
                .and_then(|(b, _)| tdb_speech::parse_evt0(&b).ok())
                .and_then(|e| e.params.iter().find(|q| q.name == "playerid").map(|q| q.id))
                .unwrap_or(ANNOUNCER_PLAYER_TAG);
            let value = announcer_audio_id.and_then(|a| u32::try_from(a).ok()).unwrap_or(0);
            let (t, d, r) = name_call_tables(&ann_grf, &ann_bank, tag, value);
            if announcer_audio_id.is_none() {
                notes.push("this title's player bio table has no audio id column".into());
            } else if value == 0 {
                notes.push("audio id 0: no announcer name id for this player".into());
            } else if t.is_empty() {
                notes.push(format!("no announcer table is keyed by audio id {value}: no announcer recording for this player (8–30 % of a roster per title, handoff 07 §2)"));
            }
            (Some(tag), t, d, r)
        }
        _ => {
            notes.push("no announcer module (ann_eng_*) in this title's audio/speech; the announcer join is not listed".into());
            (None, Vec::new(), 0, 0)
        }
    };
    Ok(PlayerNameCalls {
        player_id,
        first_name,
        last_name,
        pbp_names,
        pbp_player_tag,
        announcer_audio_id,
        announcer_tag,
        announcer_tables,
        announcer_sample_ids_distinct: ann_distinct,
        announcer_bank_resolved_distinct: ann_resolved,
        notes,
    })
}

// ------------------------------------------------------------------ Phase 1.5 / 2.7: context records

/// The read-only context listing (research 2026-09-21 handoff 01, contract rev. 2 of handoff
/// 03): which game trigger starts a context, the named conditions it tests, and what it does —
/// speak a speech event or raise a context trigger. Wording: a context "speaks" an event through
/// its item; which context the engine picks among triggered candidates, the bytecode the
/// conditions evaluate and the undecoded header words are not claimed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextCatalogueView {
    pub game_id: String,
    pub platform: String,
    pub context_dir: String,
    pub records: usize,
    pub triggers: usize,
    pub condition_names: usize,
    pub speech_items: usize,
    pub context_items: usize,
    pub query: String,
    pub matching: usize,
    pub truncated: bool,
    pub contexts: Vec<ContextRecordView>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextRecordView {
    pub index: u32,
    pub name: String,
    pub trigger: Option<String>,
    pub conditions: Vec<String>,
    pub items: Vec<ContextItemView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextItemView {
    /// `speech` / `context` / `unknown-namespace`.
    pub kind: String,
    pub name: String,
    pub event_id: u16,
    /// Parameter values as text: an immediate as its number, a call block as `blob N(args)`.
    pub params: Vec<String>,
}

fn context_coverage(game_id: &str, platform: crate::Platform) -> std::result::Result<(), String> {
    match (game_id, platform) {
        ("nhl13" | "nhl14" | "nhl15" | "nhllegacy" | "nhllegacy-recompiled" | "legacyrecomp", crate::Platform::Xbox | crate::Platform::Recomp) => Ok(()),
        ("nhl14" | "nhl15" | "nhllegacy", crate::Platform::Ps3) => Ok(()),
        ("nhl11" | "nhl12", crate::Platform::Xbox | crate::Platform::Ps3) => Ok(()),
        ("nhl13", crate::Platform::Ps3) => Err("NHL 13 PS3 ships its context tables only inside audioboot.big; the archive route is not built".into()),
        // PS4 NHL 15–21: the `0XTC` walk of research 2026-09-21 pm handoff 15. The *speech
        // catalogue* is still a separate package on PS4 (`coverage` above refuses it) — only the
        // contexts listing is covered here, which is why this arm sits in `context_coverage` and
        // not in `coverage`.
        ("nhl15" | "nhl16" | "nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21", crate::Platform::Ps4) => Ok(()),
        _ => coverage(game_id, platform),
    }
}

/// The PS4 speech directory: `<nhl>/data/kettle/audio/speech/eng`, derived from the same anchor
/// walk the art roots use, so a discovery that pointed deep into the database tree still resolves.
fn ps4_speech_dir(install_root: &Path) -> PathBuf {
    crate::assets::ps4_kettle_root(install_root).join("audio").join("speech").join("eng")
}

/// Build the PS4 context listing (`0XTC`). Same shape as the gen7 listing so the card is shared.
fn ps4_context_catalogue(game_id: &str, install_root: &Path, query: &str, limit: usize) -> Result<ContextCatalogueView> {
    let dir = ps4_speech_dir(install_root);
    if !dir.is_dir() {
        return Err(DomainError::Validation(format!("{}: no extracted kettle speech directory under this root (PS4 contexts are read from the extracted tree)", dir.display())));
    }
    let read = |n: &str| std::fs::read(dir.join(n)).map_err(|e| DomainError::Validation(format!("{}: {e}", dir.join(n).display())));
    let bad = |what: &str, e: tdb_speech::SpeechError| DomainError::Validation(format!("{}: {what}: {e}", dir.display()));
    let sm = tdb_speech::parse_moir_le(&read("pbp_speecheventdef.csi")?).map_err(|e| bad("speech names", e))?;
    let cm = tdb_speech::parse_moir_le(&read("pbp_contexteventdef.csi")?).map_err(|e| bad("context names", e))?;
    let bytes = read("pbp_contexts.bin")?;
    let (ctx, notes) = tdb_speech::parse_ctx0_ps4(&bytes, Some(&sm), Some(&cm)).map_err(|e| bad("contexts", e))?;
    let mut view = context_view(game_id, "ps4", &dir, ctx, query, limit);
    view.notes.push(format!(
        "PS4 `0XTC` version {} ({} groups, {} undecoded 16-byte block entries): the record walk of research 2026-09-21 pm handoff 15. Condition argument *values* and the `.gas` bytecode are not decoded.",
        notes.version, notes.groups, notes.block_entries
    ));
    Ok(view)
}

/// Build the context listing. Gen7 (NHL 11–Legacy) reads the loose `audio/context` +
/// `audio/speech` files; **PS4 (NHL 15–21)** reads the `0XTC` container in the extracted kettle
/// tree (research 2026-09-21 pm handoff 15) and produces the same view — the card is shared.
pub fn context_catalogue(game_id: &str, platform: crate::Platform, install_root: &Path, query: &str, limit: usize) -> Result<ContextCatalogueView> {
    context_coverage(game_id, platform).map_err(DomainError::Validation)?;
    if platform == crate::Platform::Ps4 {
        return ps4_context_catalogue(game_id, install_root, query, limit);
    }
    // Receipt 67: loose first; Legacy Recompiled reads its archive members when the loose trees are absent (handoff 16).
    let ctx_src = AudioDir::open(game_id, platform, install_root, "audio/context")?;
    let speech_src = AudioDir::open(game_id, platform, install_root, "audio/speech")?;
    let read = |src: &AudioDir, name: &str| src.read(name, true).map(|o| o.expect("required").0);
    let dir = PathBuf::from(ctx_src.describe());
    let bad = |what: &str, e: tdb_speech::SpeechError| DomainError::Validation(format!("{}: {what}: {e}", dir.display()));
    let bytes = read(&ctx_src, "pbp_eng_contexts.bin")?;
    let ctx = if tdb_speech::is_ctx0_v31(&bytes) {
        // NHL 11 / 12 (research 2026-09-21 handoff 16): items typed by the MOIR name tables.
        let sm = tdb_speech::parse_moir(&read(&speech_src, "pbp_eng_speecheventdef.csi")?).map_err(|e| bad("speech names", e))?;
        let cm = tdb_speech::parse_moir(&read(&ctx_src, "pbp_eng_contexteventdef.csi")?).map_err(|e| bad("context names", e))?;
        tdb_speech::parse_ctx0_v31(&bytes, Some(&sm), Some(&cm)).map_err(|e| bad("contexts (v3.1)", e))?
    } else {
        let speech_ns = tdb_speech::parse_evt0(&read(&speech_src, "pbp_eng_speecheventsubscr.bin")?).map_err(|e| bad("speech events", e))?.namespace;
        let ctx_ns = tdb_speech::parse_evt0(&read(&ctx_src, "pbp_eng_contexteventsubscr.bin")?).map_err(|e| bad("context events", e))?.namespace;
        tdb_speech::parse_ctx0(&bytes, Some(speech_ns), Some(ctx_ns)).map_err(|e| bad("contexts", e))?
    };
    let mut view = context_view(game_id, platform.as_str(), &dir, ctx, query, limit);
    if matches!(ctx_src, AudioDir::Members { .. }) {
        view.notes.push(MEMBER_NOTE.into());
    }
    Ok(view)
}

/// Turn a parsed `Ctx0` into the listing view: the query filter, the per-record rows and the
/// counts. Shared by the gen7 and PS4 routes so both cards read identically.
fn context_view(game_id: &str, platform: &str, dir: &Path, ctx: tdb_speech::Ctx0, query: &str, limit: usize) -> ContextCatalogueView {
    let q = query.trim().to_ascii_lowercase();
    let hit = |r: &tdb_speech::CtxRecord| -> bool {
        q.is_empty()
            || r.name.to_ascii_lowercase().contains(&q)
            || r.trigger.as_deref().is_some_and(|t| t.to_ascii_lowercase().contains(&q))
            || r.items.iter().any(|i| i.name.to_ascii_lowercase().contains(&q))
            || r.conditions.iter().any(|c| c.name.to_ascii_lowercase().contains(&q))
    };
    let matching = ctx.records.iter().filter(|r| hit(r)).count();
    let contexts: Vec<ContextRecordView> = ctx
        .records
        .iter()
        .filter(|r| hit(r))
        .take(limit)
        .map(|r| ContextRecordView {
            index: r.index,
            name: r.name.clone(),
            trigger: r.trigger.clone(),
            conditions: r.conditions.iter().map(|c| if c.args.is_empty() { c.name.clone() } else { format!("{}({})", c.name, c.args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ")) }).collect(),
            items: r
                .items
                .iter()
                .map(|i| ContextItemView {
                    kind: i.kind.to_string(),
                    name: i.name.clone(),
                    event_id: i.event_id,
                    params: i
                        .params
                        .iter()
                        .map(|p| match (p.kind, p.value, p.blob) {
                            (0, Some(v), _) => format!("trigger slot {v}"),
                            (1, Some(v), _) => v.to_string(),
                            (2, _, Some(b)) => format!("script {b}({})", p.args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ")),
                            _ => "?".into(),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect();
    let (mut speech_items, mut context_items) = (0usize, 0usize);
    for r in &ctx.records {
        speech_items += r.items.iter().filter(|i| i.kind == "speech").count();
        context_items += r.items.iter().filter(|i| i.kind == "context").count();
    }
    ContextCatalogueView {
        game_id: game_id.to_string(),
        platform: platform.to_string(),
        context_dir: dir.display().to_string(),
        records: ctx.records.len(),
        triggers: ctx.trigger_names.len(),
        condition_names: ctx.condition_names,
        speech_items,
        context_items,
        query: query.to_string(),
        matching,
        truncated: matching > contexts.len(),
        contexts,
        notes: vec![
            "read-only: each context names the trigger that starts its evaluation, the conditions it tests (named compiled expressions; what they compute is not decoded) and its items — a speech event it speaks or a context trigger it raises. Which context the engine picks among triggered candidates, its priority words and the run-once groups are runtime".to_string(),
            format!("{} contexts have no trigger (evaluated by another path — not decoded)", ctx.records.iter().filter(|r| r.trigger.is_none()).count()),
        ],
    }
}
