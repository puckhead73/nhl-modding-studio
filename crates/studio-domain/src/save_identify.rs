//! Save Editor file identification: given a save container or an extracted database, establish
//! what the file is (container format, save kind, writability) and which bundled game target's
//! metadata describes it.
//!
//! The evidence is deliberately layered and reported, never blended into a silent guess:
//!
//! 1. **Byte order** separates PS4 (little-endian) from PS3 / Xbox 360 / Recompiled (big-endian).
//! 2. **Schema compatibility** — every table and field the file carries must be declared by the
//!    target's bundled metadata at the same bit width (the rule [`crate::Project::validate_metadata_schema`]
//!    applies). An incompatible target is never a candidate.
//! 3. **Exact schema match** — the file's table/field layout equals the target's own anchor main
//!    database or anchor roster save. Neighbouring titles often share most of a schema; an exact
//!    match is stronger evidence than compatibility alone.
//! 4. **Title ids in the path** — a PS3 product code (`BLUS31179…`), an Xbox 360 title id
//!    (`454109BE`) or a PS4 content id (`CUSA…`) in the file's own path names the game and platform.
//!
//! When the evidence leaves more than one game or platform standing, the result says so and the
//! caller asks the user to choose — titles with an identical schema cannot be told apart by schema.
//! NHL 07 / NHL 08 roster *change lists* (a save that records changes against the game database
//! rather than holding the roster) are recognised and opened applied to the game's database
//! ([`crate::change_list_overlay`]).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tdb_core::{Endian, TdbFile};
use tdb_defs::FieldMetaIndex;

use crate::baselines::{self, BaselineStore, TargetBaseline};
use crate::{DomainError, Platform, Result};

/// A database layout: table id â†’ (field id â†’ bit width).
pub type SchemaSignature = BTreeMap<String, BTreeMap<String, u32>>;

/// The layout of a parsed TDB.
pub fn schema_signature(db: &TdbFile<'_>) -> SchemaSignature {
    db.tables
        .iter()
        .map(|t| {
            (
                t.id.raw(),
                t.fields.iter().map(|f| (f.id.raw(), f.bit_width)).collect(),
            )
        })
        .collect()
}

/// `true` when every table and field of `sig` is declared by `meta` at the same bit width.
pub fn metadata_covers(meta: &FieldMetaIndex, sig: &SchemaSignature) -> bool {
    sig.iter().all(|(table, fields)| {
        fields.iter().all(|(field, width)| {
            meta.field(table, field)
                .is_some_and(|m| m.bit_depth == *width)
        })
    })
}

/// How the file holds its database.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileFormat {
    /// A bare TDB database (an extracted `nhlng.db`, `nhl_game.db` or an extracted save body).
    Database,
    /// A PS3 / Xbox 360 save container (`SYS-DATA`, or the Xbox save file).
    Container,
    /// A PS4 save `DATA` stream.
    Ps4Save,
}

/// What the file is, independent of which game it belongs to.
#[derive(Clone, Debug, Serialize)]
pub struct FileProbe {
    pub format: FileFormat,
    /// `v1` | `v2` | `v3` | `v4` | `ps4` for saves; `None` for a bare database.
    pub container_version: Option<String>,
    /// `roster` | `beagm` | `beapro` | `season` | `hockeycard` | `settings` | `unknown`, or
    /// `database` for a bare database.
    pub kind: String,
    /// Both stored container checksums match (`None` where the format carries none we verify).
    pub checksums_valid: Option<bool>,
    /// The codec can write this container back.
    pub writable: bool,
    /// The write path for this container variant is not verified in-game.
    pub write_experimental: bool,
    /// PS4 databases are little-endian; PS3 / Xbox 360 / Recompiled are big-endian.
    pub little_endian: bool,
    pub table_count: usize,
}

/// One bundled target that can describe the file.
#[derive(Clone, Debug, Serialize)]
pub struct Candidate {
    pub game_id: String,
    pub platform: Platform,
    pub title: String,
    /// The file's layout equals this target's anchor main database or anchor roster save.
    pub exact_schema: bool,
    /// A title id in the file's path names this game.
    pub path_names_game: bool,
    /// A title id in the file's path names this platform.
    pub path_names_platform: bool,
    /// Plain-language reasons this candidate is listed.
    pub evidence: Vec<String>,
    pub score: u32,
}

/// The identification of one file.
#[derive(Clone, Debug, Serialize)]
pub struct Identification {
    pub path: PathBuf,
    pub probe: FileProbe,
    /// Compatible bundled targets, best first.
    pub candidates: Vec<Candidate>,
    /// The best candidate is the only one at its evidence level for both game and platform.
    pub game_confident: bool,
    pub platform_confident: bool,
    /// The file is an NHL 07-style roster change list against the identified game's database.
    pub change_list: bool,
    /// Title ids found in the path (diagnostic; shown behind disclosure).
    pub path_hints: Vec<String>,
    /// Why the editor's team/player model cannot be built over this file (a required table or
    /// field is absent), if it cannot — checked after supplementing a save from its game's
    /// database.
    pub entity_problem: Option<String>,
    /// Why a change list could not be applied to its game's database, if it could not.
    pub change_list_problem: Option<String>,
    /// Tables the file itself carries (editable); for a change list, the game tables it can
    /// record changes to.
    pub own_tables: usize,
    /// Tables a save lacks that are shown read-only from the leading candidate's game database
    /// (0 for a bare database, which is never supplemented).
    pub supplied_tables: usize,
}

/// Read and classify a file's container without consulting any metadata.
pub fn probe_bytes(bytes: Vec<u8>) -> Result<(FileProbe, Vec<u8>)> {
    let header = tdb_savedata::SaveContainer::parse(&bytes).ok().map(|c| {
        (
            c.header.version,
            c.header.save_type,
            c.checksums_valid(),
            c.writeable(),
        )
    });
    let (source, inner) = crate::project::detect_source_for_check(bytes)?;
    let db = TdbFile::parse(&inner)?;
    let (endian, table_count) = (db.endian, db.tables.len());
    let probe = match source {
        crate::Source::RawDb => FileProbe {
            format: FileFormat::Database,
            container_version: None,
            kind: "database".into(),
            checksums_valid: None,
            writable: true,
            write_experimental: false,
            little_endian: endian == Endian::Little,
            table_count,
        },
        crate::Source::Container { .. } => {
            let (version, save_type, checks, writable) = header.ok_or(DomainError::NotADatabase)?;
            FileProbe {
                format: FileFormat::Container,
                container_version: Some(
                    match version {
                        tdb_savedata::ContainerVersion::RawV1 => "v1",
                        tdb_savedata::ContainerVersion::RawV2 => "v2",
                        tdb_savedata::ContainerVersion::RawV3 => "v3",
                        tdb_savedata::ContainerVersion::ZlibV4 => "v4",
                    }
                    .into(),
                ),
                kind: save_kind(save_type).into(),
                checksums_valid: Some(checks),
                writable,
                write_experimental: version.write_is_experimental(),
                little_endian: endian == Endian::Little,
                table_count,
            }
        }
        crate::Source::Ps4Save { .. } => FileProbe {
            format: FileFormat::Ps4Save,
            container_version: Some("ps4".into()),
            kind: "roster".into(),
            checksums_valid: None,
            writable: true,
            // The PS4 repack is structurally checked only; on-console load is unverified.
            write_experimental: true,
            little_endian: endian == Endian::Little,
            table_count,
        },
    };
    Ok((probe, inner))
}

/// The database layout of the file at `path` (container unwrapped).
pub fn file_signature(path: &Path) -> Result<SchemaSignature> {
    let bytes = std::fs::read(path).map_err(|source| DomainError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let (_, inner) = probe_bytes(bytes)?;
    Ok(schema_signature(&TdbFile::parse(&inner)?))
}

fn save_kind(t: tdb_savedata::SaveType) -> &'static str {
    use tdb_savedata::SaveType as S;
    match t {
        S::Roster => "roster",
        S::Settings => "settings",
        S::BeAGm => "beagm",
        S::BeAPro => "beapro",
        S::HockeyCard => "hockeycard",
        S::Season => "season",
        S::Unknown => "unknown",
    }
}

/// The schema evidence of one bundled target, computed once per baseline store.
struct TargetSchema {
    game_id: String,
    platform: Platform,
    title: String,
    endian: Option<Endian>,
    meta: Arc<FieldMetaIndex>,
    /// Main database bytes (kept for change-list recognition) and layout.
    main: Option<(Arc<Vec<u8>>, SchemaSignature)>,
    roster: Option<SchemaSignature>,
}

/// Every available bundled target's metadata and anchor layouts. Building it reads and verifies
/// each target's metadata, main database and roster save once; reuse it across identifications.
pub struct SchemaIndex {
    inventory_path: PathBuf,
    targets: Vec<TargetSchema>,
}

impl SchemaIndex {
    pub fn build(store: &BaselineStore) -> Result<Self> {
        // Targets sharing a component (Recompiled reuses the Xbox Legacy files) share the parse.
        let mut metas: HashMap<String, Arc<FieldMetaIndex>> = HashMap::new();
        let mut mains: HashMap<String, Option<(Arc<Vec<u8>>, SchemaSignature, Endian)>> =
            HashMap::new();
        let mut targets = Vec::new();
        for t in store.inventory.targets.iter().filter(|t| t.is_available()) {
            let Some(platform) = platform_of(t) else {
                continue;
            };
            let Some(meta_c) = t.component(baselines::ROLE_META) else {
                continue;
            };
            let meta = match metas.get(&meta_c.path) {
                Some(m) => m.clone(),
                None => {
                    let bytes = store.read_verified(meta_c)?;
                    let m = Arc::new(parse_meta(&bytes, &meta_c.path)?);
                    metas.insert(meta_c.path.clone(), m.clone());
                    m
                }
            };
            let main = match t.component(baselines::ROLE_MAIN_DB) {
                Some(c) => mains
                    .entry(c.path.clone())
                    .or_insert_with(|| {
                        let bytes = store.read_verified(c).ok()?;
                        let db = TdbFile::parse(&bytes).ok()?;
                        let (sig, endian) = (schema_signature(&db), db.endian);
                        Some((Arc::new(bytes), sig, endian))
                    })
                    .clone(),
                None => None,
            };
            let roster = t
                .component(baselines::ROLE_ROSTER_SAVE)
                .and_then(|c| store.read_verified(c).ok())
                .and_then(|b| probe_bytes(b).ok())
                .and_then(|(_, inner)| TdbFile::parse(&inner).ok().map(|db| schema_signature(&db)));
            targets.push(TargetSchema {
                game_id: t.game_id.clone(),
                platform,
                title: t.title.clone(),
                endian: main.as_ref().map(|m| m.2),
                meta,
                main: main.map(|(b, s, _)| (b, s)),
                roster,
            });
        }
        Ok(SchemaIndex {
            inventory_path: store.inventory_path.clone(),
            targets,
        })
    }

    /// The inventory this index was built from (a cache key).
    pub fn inventory_path(&self) -> &Path {
        &self.inventory_path
    }

    /// The bundled metadata for a target, if it is indexed.
    pub fn meta(&self, game_id: &str, platform: Platform) -> Option<Arc<FieldMetaIndex>> {
        self.targets
            .iter()
            .find(|t| t.game_id == game_id && t.platform == platform)
            .map(|t| t.meta.clone())
    }

    /// Identify the file at `path`.
    pub fn identify(&self, path: &Path) -> Result<Identification> {
        let bytes = std::fs::read(path).map_err(|source| DomainError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let (probe, inner) = probe_bytes(bytes)?;
        let db = TdbFile::parse(&inner)?;
        let sig = schema_signature(&db);
        let hints = path_hints(path);

        let mut change_list = false;
        let mut candidates = Vec::new();
        for t in &self.targets {
            if t.endian.is_some_and(|e| e != db.endian) || t.platform.endian() != db.endian {
                continue;
            }
            let mut evidence = Vec::new();
            let mut exact = false;
            let mut is_change_list = false;
            if metadata_covers(&t.meta, &sig) {
                exact = t.main.as_ref().is_some_and(|(_, s)| *s == sig)
                    || t.roster.as_ref().is_some_and(|s| *s == sig);
                evidence.push(if exact {
                    format!("Its database layout matches {} exactly.", t.title)
                } else {
                    format!(
                        "{}'s metadata describes every table and field it contains.",
                        t.title
                    )
                });
            } else if probe.kind == "roster"
                && t.main.as_ref().is_some_and(|(base, _)| {
                    crate::roster_difflist::recognize(base, &inner, &t.meta).is_ok()
                })
            {
                is_change_list = true;
                evidence.push(format!(
                    "It records roster changes against {}'s game database.",
                    t.title
                ));
            } else {
                continue;
            }
            let names_game = hints
                .iter()
                .any(|h| family(&h.game_id) == family(&t.game_id));
            // Recompiled runs the Xbox 360 Legacy data, so an Xbox Legacy title id in the path
            // names both runtimes equally; it never decides between them.
            let names_platform = hints.iter().any(|h| {
                h.game_id == t.game_id && h.platform == t.platform
                    || t.platform == Platform::Recomp
                        && h.platform == Platform::Xbox
                        && family(&h.game_id) == family(&t.game_id)
            });
            if names_platform {
                evidence.push(format!("Its folder carries {}'s title id.", t.title));
            }
            change_list |= is_change_list;
            let score = u32::from(names_platform) * 100
                + u32::from(names_game) * 40
                + u32::from(exact || is_change_list) * 20
                + 10;
            candidates.push(Candidate {
                game_id: t.game_id.clone(),
                platform: t.platform,
                title: t.title.clone(),
                exact_schema: exact,
                path_names_game: names_game,
                path_names_platform: names_platform,
                evidence,
                score,
            });
        }
        // A change list is only ever read against its own title; drop plain-compatibility noise.
        if change_list {
            candidates.retain(|c| {
                c.evidence
                    .iter()
                    .any(|e| e.starts_with("It records roster changes"))
            });
        }
        candidates.sort_by(|a, b| b.score.cmp(&a.score));
        let top = candidates.first().map(|c| c.score);
        let leaders: Vec<&Candidate> = candidates.iter().filter(|c| Some(c.score) == top).collect();
        let game_confident = !leaders.is_empty()
            && leaders
                .iter()
                .all(|c| family(&c.game_id) == family(&leaders[0].game_id));
        let platform_confident = leaders.len() == 1;
        // A save is shown over its game's database: the editor's model must build over that
        // composite (a bare database is never supplemented).
        let base = candidates.first().and_then(|c| {
            self.targets
                .iter()
                .find(|t| t.game_id == c.game_id && t.platform == c.platform)
                .and_then(|t| t.main.as_ref().map(|(b, _)| b.clone()))
        });
        let mut own_tables = db.tables.len();
        let mut supplied_tables = 0;
        let mut change_list_problem = None;
        let composite = if change_list {
            // A change list is shown applied to its own game's database.
            let leader = candidates.first().and_then(|c| {
                self.targets
                    .iter()
                    .find(|t| t.game_id == c.game_id && t.platform == c.platform)
            });
            match leader.and_then(|t| t.main.as_ref().map(|(b, _)| (t, b.clone()))) {
                Some((t, base)) => match crate::change_list_overlay::compose(&inner, base, &t.meta) {
                    Ok((composite, overlay)) => {
                        own_tables = overlay.editable_tables();
                        supplied_tables = TdbFile::parse(&composite)?.tables.len() - own_tables;
                        Some(composite)
                    }
                    Err(e) => {
                        change_list_problem = Some(e.to_string());
                        None
                    }
                },
                None => None,
            }
        } else {
            match (&probe.format, base) {
                (FileFormat::Database, _) | (_, None) => None,
                (_, Some(base)) => crate::table_overlay::compose(&inner, base)?.map(|(c, o)| {
                    supplied_tables = o.frozen.len();
                    c
                }),
            }
        };
        let entity_problem = if change_list_problem.is_some() {
            None
        } else {
            match &composite {
                Some(c) => tdb_relations::RosterModel::build(&TdbFile::parse(c)?).err(),
                None => tdb_relations::RosterModel::build(&db).err(),
            }
            .map(|e| e.to_string())
        };
        let mut probe = probe;
        if let Some(kind) = refined_kind(&probe, path) {
            probe.kind = kind.into();
        }
        Ok(Identification {
            path: path.to_path_buf(),
            probe,
            candidates,
            game_confident,
            platform_confident,
            change_list,
            path_hints: hints.iter().map(|h| h.code.clone()).collect(),
            entity_problem,
            change_list_problem,
            own_tables,
            supplied_tables,
        })
    }
}

/// The game a target id belongs to: Legacy Recompiled is NHL Legacy on another runtime.
fn family(game_id: &str) -> &str {
    if game_id == "legacyrecomp" {
        "nhllegacy"
    } else {
        game_id
    }
}

/// A better save kind than the container header gives, where one is available: a PS4 save's
/// `NAME` file (`RosterCustom0`, `BeAGM1`, `Season0`, `ThreeOnThree0`) — the PS4 codec decodes
/// every mode save through the same roster record — or, for a container whose magic is not
/// recognised, the save's own file/folder name (`BLUS…00 - DYNASTY`, `SEASON1 2026…`).
fn refined_kind(probe: &FileProbe, path: &Path) -> Option<&'static str> {
    let from_name = |name: &str| -> Option<&'static str> {
        let n = name.to_ascii_uppercase();
        [
            ("SETTINGS", "settings"),
            ("PROFILE", "settings"),
            ("HOCKEYCARD", "hockeycard"),
            ("ROSTER", "roster"),
            ("DYNASTY", "beagm"),
            ("BEAGM", "beagm"),
            ("FRANCHISE", "beagm"),
            ("SEASON", "season"),
            ("BEAPRO", "beapro"),
            ("THREEONTHREE", "threes"),
        ]
        .into_iter()
        .find(|(token, _)| n.contains(token))
        .map(|(_, kind)| kind)
    };
    let names = || {
        path.ancestors()
            .take(2)
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    };
    match probe.format {
        FileFormat::Ps4Save => {
            let name_file = path.parent().map(|d| d.join("NAME"));
            let declared = name_file.and_then(|p| std::fs::read(p).ok()).map(|b| {
                String::from_utf8_lossy(&b)
                    .trim_end_matches('\0')
                    .trim()
                    .to_string()
            });
            declared
                .as_deref()
                .and_then(from_name)
                .or_else(|| names().iter().find_map(|n| from_name(n)))
        }
        FileFormat::Container if probe.kind == "unknown" => {
            names().iter().find_map(|n| from_name(n))
        }
        _ => None,
    }
}

/// A plain-language name for a save kind.
pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "database" => "Game database",
        "roster" => "Roster save",
        "beagm" => "Franchise / Be a GM save",
        "season" => "Season save",
        "beapro" => "Be a Pro save",
        "threes" => "3-on-3 save",
        "hockeycard" => "Ultimate Team save",
        "settings" => "Settings / profile save",
        _ => "Unidentified save",
    }
}

/// Whether the Save Editor may open an identified file, and what it must tell the user first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", content = "reason", rename_all = "snake_case")]
pub enum OpenPolicy {
    Allowed,
    /// Openable after the user acknowledges the reason.
    Caution(String),
    Refused(String),
}

impl Identification {
    /// The Save Editor's decision for this file. Writing caveats that do not block opening (an
    /// experimental container write) are reported separately by [`Identification::notes`].
    pub fn open_policy(&self) -> OpenPolicy {
        let kind = self.probe.kind.as_str();
        if let Some(problem) = &self.change_list_problem {
            return OpenPolicy::Refused(problem.clone());
        }
        match kind {
            "settings" | "hockeycard" => {
                return OpenPolicy::Refused(format!(
                    "This is a {}. It holds game settings or Ultimate Team progress rather than roster data, which Save Editor does not edit.",
                    kind_label(kind).to_lowercase()
                ))
            }
            _ => {}
        }
        if self.entity_problem.is_some() {
            return OpenPolicy::Refused(format!(
                "This {} does not hold the complete team and player tables Save Editor edits, so it cannot be opened. It may store only part of a game mode's data.",
                kind_label(kind).to_lowercase()
            ));
        }
        if self.candidates.is_empty() {
            return OpenPolicy::Refused(
                "None of Studio's bundled game definitions describe this file's database. If you have the game's matching metadata file (for example nhl_game-meta.xml), choose it under Advanced and pick the game yourself.".into(),
            );
        }
        if !self.probe.writable {
            return OpenPolicy::Caution(
                "Studio can read this save format but cannot write it back. It opens for viewing only; changes cannot be saved.".into(),
            );
        }
        match kind {
            "database" | "roster" => OpenPolicy::Allowed,
            "beagm" | "season" | "beapro" | "threes" => OpenPolicy::Caution(format!(
                "This is a {}. It stores its game mode's own data rather than complete rosters, so the teams and players it does not contain are shown from the game's default database and cannot be changed. Save Editor does not keep the mode's progress (schedules played, standings, finances) consistent with any change you make, so editing this save is experimental. Keep a copy of the original.",
                kind_label(kind).to_lowercase()
            )),
            _ => OpenPolicy::Caution(
                "Studio could not tell what kind of save this is. It contains a game database that the identified game's definitions describe, but editing it is experimental. Keep a copy of the original.".into(),
            ),
        }
    }

    /// Non-blocking facts the user should see before saving.
    pub fn notes(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.probe.write_experimental {
            out.push(match self.probe.container_version.as_deref() {
                Some("v1") => "NHL 10 and NHL 11 saves carry an integrity value Studio cannot recompute yet. Saving keeps the original value, and whether the game accepts the edited save has not been verified.".into(),
                Some("ps4") => "Writing PS4 saves is structurally checked only; loading an edited save on a console or emulator has not been verified.".into(),
                _ => "Writing this save format has not been verified in-game.".into(),
            });
        }
        if self.change_list {
            out.push(format!(
                "This roster save stores changes to the game's own database rather than whole rosters (the NHL 07 and NHL 08 save model). Studio shows the game's database with the save's changes applied. The {} tables the save can record changes to (players, roster assignments, free agents and their ratings) are editable; the other {} are read-only.",
                self.own_tables, self.supplied_tables
            ));
            match self.candidates.first().map(|c| c.game_id.as_str()) {
                Some("nhl07") => out.push("Jersey numbers and skater ratings saved by Studio have loaded correctly in NHL 07. Other kinds of change follow the game's own save definitions but have not been checked in-game yet.".into()),
                _ => out.push("Saving follows the game's own save definitions, but loading a roster save edited by Studio has not been checked in this game yet.".into()),
            }
        } else if self.supplied_tables > 0 {
            out.push(format!(
                "This save contains {} of the game's database tables. The other {} are shown from Studio's copy of the game's default database so every screen is complete; they are read-only and are never written into the save.",
                self.own_tables, self.supplied_tables
            ));
        }
        if self.probe.checksums_valid == Some(false) {
            out.push("The save's stored checksums do not match its contents. It may have been edited by another tool; saving recomputes them.".into());
        }
        out
    }
}

fn platform_of(t: &TargetBaseline) -> Option<Platform> {
    match t.platform.as_str() {
        "ps3" => Some(Platform::Ps3),
        "xbox" => Some(Platform::Xbox),
        "ps4" => Some(Platform::Ps4),
        "recomp" => Some(Platform::Recomp),
        _ => None,
    }
}

/// Parse metadata bytes, decoding the RefPack some shipped discs keep it in.
pub fn parse_meta(bytes: &[u8], label: &str) -> Result<FieldMetaIndex> {
    let bytes = if tdb_archive_codec::is_refpack(bytes) {
        tdb_archive_codec::refpack_decompress(bytes)
            .map_err(|e| DomainError::Validation(format!("{label}: metadata RefPack: {e}")))?
    } else {
        bytes.to_vec()
    };
    let text = String::from_utf8(bytes)
        .map_err(|_| DomainError::Validation(format!("{label} is not readable metadata text")))?;
    Ok(FieldMetaIndex::from_xml(&text)?)
}

/// A title id found in a file's path.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PathHint {
    code: String,
    game_id: String,
    platform: Platform,
}

const HINT_GAMES: [&str; 17] = [
    "nhl07",
    "nhl08",
    "nhl09",
    "nhl10",
    "nhl11",
    "nhl12",
    "nhl13",
    "nhl14",
    "nhl15",
    "nhllegacy",
    "nhl16",
    "nhl17",
    "nhl18",
    "nhl19",
    "nhl20",
    "nhl21",
    "legacyrecomp",
];

/// Title ids in the path's components (the file name and up to six enclosing folders): RPCS3
/// save folders start with the PS3 product code, Xenia content trees carry the Xbox title id as a
/// folder, and PS4 save trees carry the `CUSA` id.
fn path_hints(path: &Path) -> Vec<PathHint> {
    let parts: Vec<String> = path
        .ancestors()
        .take(7)
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_ascii_uppercase())
        .collect();
    let mut out: Vec<PathHint> = Vec::new();
    let mut push = |code: &str, game: &str, platform: Platform| {
        let h = PathHint {
            code: code.into(),
            game_id: game.into(),
            platform,
        };
        if !out.contains(&h) {
            out.push(h);
        }
    };
    for game in HINT_GAMES {
        if let Some(ids) = crate::discovery::title_ids(game) {
            for code in [ids.ps3_us, ids.ps3_eu]
                .into_iter()
                .filter(|c| !c.is_empty())
            {
                if parts.iter().any(|p| p.starts_with(code)) {
                    push(code, game, Platform::Ps3);
                }
            }
            if !ids.xbox.is_empty() && parts.iter().any(|p| p == ids.xbox) {
                push(ids.xbox, game, Platform::Xbox);
            }
        }
        for code in crate::discovery::ps4_title_ids(game) {
            if parts.iter().any(|p| p.contains(code)) {
                push(code, game, Platform::Ps4);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_hints_read_product_codes_title_ids_and_content_ids() {
        let ps3 = path_hints(Path::new(
            "C:/rpcs3/dev_hdd0/home/00000001/savedata/BLUS311790200 - ROSTER/SYS-DATA",
        ));
        assert_eq!(ps3.len(), 1);
        assert_eq!(
            (ps3[0].game_id.as_str(), ps3[0].platform),
            ("nhl14", Platform::Ps3)
        );
        let xbox = path_hints(Path::new(
            "D:/xenia/content/0000000000000000/454109EC/00000001/ROSTER 20260101/ROSTER",
        ));
        assert_eq!(
            (xbox[0].game_id.as_str(), xbox[0].platform),
            ("nhllegacy", Platform::Xbox)
        );
        let ps4 = path_hints(Path::new(
            "E:/shadps4/user/savedata/1/CUSA18182/RosterCustom0/DATA",
        ));
        assert_eq!(
            (ps4[0].game_id.as_str(), ps4[0].platform),
            ("nhl21", Platform::Ps4)
        );
        assert!(path_hints(Path::new("C:/Users/me/Desktop/nhlng.db")).is_empty());
    }
}
