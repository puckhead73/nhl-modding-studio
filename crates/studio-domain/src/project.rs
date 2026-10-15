//! [`Project`] — a loaded database plus its registries, tagged with its target `(platform, role)`.

use std::fs;
use std::path::Path;

use tdb_core::{Endian, TdbFile};
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::RelationRegistry;
use tdb_savedata::SaveContainer;

use crate::error::DomainError;
use crate::Result;

/// The console/runtime a database targets. Selects byte order in the codec layer; PS3/Xbox/Recomp
/// are all big-endian, PS4 is little-endian (Phase 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Platform {
    Ps3,
    Xbox,
    Ps4,
    Recomp,
}

impl Platform {
    /// The on-disk byte order this platform's databases use.
    pub fn endian(self) -> Endian {
        match self {
            Platform::Ps3 | Platform::Xbox | Platform::Recomp => Endian::Big,
            Platform::Ps4 => Endian::Little,
        }
    }

    /// A stable identifier for DTOs / diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Ps3 => "ps3",
            Platform::Xbox => "xbox",
            Platform::Ps4 => "ps4",
            Platform::Recomp => "recomp",
        }
    }
}

/// Which database this project represents — they share the TDB engine but differ in role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectRole {
    /// The main `nhlng.db` roster/database.
    Nhlng,
    /// A standalone ROSTER save (often wrapped in a container).
    Roster,
    /// A localization database.
    Localization,
}

impl ProjectRole {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectRole::Nhlng => "nhlng",
            ProjectRole::Roster => "roster",
            ProjectRole::Localization => "localization",
        }
    }
}

/// How the inner TDB was obtained from the file on disk — also how it is written back.
#[derive(Clone, Debug)]
pub enum Source {
    /// A bare `.db` file: the inner TDB *is* the file.
    RawDb,
    /// A save container (PS3/Xbox `SYS-DATA`): the original container bytes are kept so an edited
    /// inner db can be repacked back into the same shape.
    Container { original: Vec<u8> },
    /// A PS4 save `DATA` stream: the live roster is a compressed little-endian TDB inside one
    /// record. The original stream is kept so the edited roster can be repacked in place.
    Ps4Save { original: Vec<u8>, trailer: Vec<u8>, original_db_len: usize },
}

/// A loaded project: the inner TDB bytes plus the metadata / enum / relationship registries, tagged
/// with the platform and role it targets and the source it came from.
pub struct Project {
    pub platform: Platform,
    pub role: ProjectRole,
    source: Source,
    db_bytes: Vec<u8>,
    meta: FieldMetaIndex,
    relations: RelationRegistry,
    enums: EnumRegistry,
    /// Identity (length + SHA-256) of the on-disk file this project was loaded from, when it was
    /// loaded from a path. A transaction adapter uses it as the expected prior of the destination,
    /// so a file edited after the session opened is a conflict rather than a silently adopted
    /// baseline. `None` for projects built from bytes.
    loaded_identity: Option<tdb_txn::FileIdentity>,
    /// Per-layout league vocabularies (`defs/leagues`); empty when built from parts.
    leagues: crate::leagues::LeagueVocabularies,
    /// The studio game id this project was opened as, when known — the explicit target evidence
    /// league-vocabulary selection requires on layouts several titles share.
    game_id: Option<String>,
    /// Settings-catalogue support data (`defs/settings`); empty when built from parts.
    settings: crate::settings_catalogue::SettingsDefs,
    /// League presentation imagery support data (`defs/league_imagery.toml`); empty from parts.
    league_imagery: crate::league_imagery::LeagueImageryDefs,
    /// Shipped-font glyph coverage per title tree (`defs/fonts/glyph-coverage.toml`); empty from parts.
    glyphs: crate::font_glyphs::GlyphCoverageDefs,
    /// Shipped single-phase schedule groups by exact source (`defs/schedules/stock-partitions.json`, receipt 48).
    schedule_stock: crate::schedule_stock::StockPartitionDefs,
    /// Save Editor: the read-only tables appended from the game's database to a partial save
    /// ([`crate::table_overlay`]). `db_bytes` then holds the composite; packaging strips it.
    overlay: Option<crate::table_overlay::Overlay>,
    /// Save Editor: an NHL 07 / NHL 08 roster change list shown applied to the game's database
    /// ([`crate::change_list_overlay`]). `db_bytes` then holds the composite; packaging rebuilds
    /// the change list.
    change_list: Option<crate::change_list_overlay::ChangeListOverlay>,
}

impl Project {
    /// Load a project from disk: read the file (raw `.db` or save container), parse the inner TDB,
    /// and build the registries from the metadata XML and the `defs/` tree (`relationships/`,
    /// `enums/` subdirectories).
    pub fn load(
        db_path: impl AsRef<Path>,
        platform: Platform,
        role: ProjectRole,
        meta_xml: impl AsRef<Path>,
        defs_root: impl AsRef<Path>,
    ) -> Result<Self> {
        let db_path = db_path.as_ref();
        let file_bytes = read(db_path)?;
        let loaded_identity = tdb_txn::FileIdentity::of_bytes(&file_bytes);
        let meta_xml = meta_xml.as_ref();
        let meta_text = read_to_string(meta_xml)?;

        let defs_root = defs_root.as_ref();
        let relations = RelationRegistry::from_dir(defs_root.join("relationships"))?;
        let enums = EnumRegistry::from_dir(defs_root.join("enums"))?;
        let leagues = crate::leagues::LeagueVocabularies::from_dir(defs_root.join("leagues"))?;
        let settings = crate::settings_catalogue::SettingsDefs::load_dir(defs_root.join("settings"))?;
        let league_imagery = crate::league_imagery::LeagueImageryDefs::load(defs_root.join("league_imagery.toml"))?;
        let glyphs = crate::font_glyphs::GlyphCoverageDefs::load(defs_root.join("fonts/glyph-coverage.toml"))?;
        let schedule_stock = crate::schedule_stock::StockPartitionDefs::load(defs_root.join("schedules/stock-partitions.json"))?;
        let meta = FieldMetaIndex::from_xml(&meta_text)?;

        let mut project = Self::from_parts(file_bytes, platform, role, meta, relations, enums)?;
        project.loaded_identity = Some(loaded_identity);
        project.leagues = leagues;
        project.settings = settings;
        project.league_imagery = league_imagery;
        project.glyphs = glyphs;
        project.schedule_stock = schedule_stock;
        Ok(project)
    }

    /// Require metadata coverage before admitting a qualified default-database project.
    /// Row counts are deliberately irrelevant: imports may contain edited rosters.
    pub fn validate_metadata_schema(&self) -> Result<()> {
        let db = TdbFile::parse(&self.db_bytes)?;
        if matches!(self.role, ProjectRole::Nhlng)
            && ["ubPc", "eGlu"].iter().any(|id| db.table_canonical(id).is_none()) {
            return Err(DomainError::Validation(
                "This file does not contain the game's player and roster database. Select the main game database, not a localization database or save file.".into()));
        }
        for table in &db.tables {
            for field in &table.fields {
                let compatible = self.meta.field(&table.id.raw(), &field.id.raw())
                    .is_some_and(|m| m.bit_depth == field.bit_width);
                if !compatible {
                    return Err(DomainError::Validation(
                        "The metadata does not match this game database. Use the matching game metadata, or remove the incorrect sibling metadata file from your import copy to use Studio's bundled metadata.".into()));
                }
            }
        }
        Ok(())
    }

    /// Shipped single-phase schedule groups (receipt 48); empty when built from parts.
    pub fn schedule_stock(&self) -> &crate::schedule_stock::StockPartitionDefs {
        &self.schedule_stock
    }

    /// The identity of the file this project was loaded from (see the field).
    pub fn loaded_identity(&self) -> Option<&tdb_txn::FileIdentity> {
        self.loaded_identity.as_ref()
    }

    /// Rebind the project to a file that now holds `file_bytes` (as read back from disk after a
    /// successful save, or after a restore): the same detection a load performs, the new source
    /// shape (a repacked container becomes the container future repacks start from), the new inner
    /// baseline and the new loaded identity. The registries are kept. Returns the inner TDB bytes
    /// the file carries so the caller can reconcile its working copy against them.
    pub(crate) fn rebind_to_file(
        &mut self,
        file_bytes: Vec<u8>,
        identity: tdb_txn::FileIdentity,
    ) -> Result<&[u8]> {
        let (source, db_bytes) = detect_source(file_bytes)?;
        let db = TdbFile::parse(&db_bytes)?;
        if db.endian != self.platform.endian() {
            return Err(DomainError::EndianMismatch {
                platform: self.platform.as_str(),
                endian: db.endian,
            });
        }
        let raw_inner = self.change_list.as_ref().map(|_| db_bytes.clone());
        let db_bytes = self.compose_inner(db_bytes)?;
        if let (Some(cl), Some(inner)) = (self.change_list.as_mut(), raw_inner) {
            cl.rebase(inner);
        }
        self.source = source;
        self.db_bytes = db_bytes;
        self.loaded_identity = Some(identity);
        Ok(&self.db_bytes)
    }

    /// Save Editor: supplement this partial database with every table `base` (the game's complete
    /// database) has and it lacks, appended read-only. Returns how many tables were added (0 when
    /// none were missing). Only a freshly loaded project may be supplemented.
    pub fn supplement_from(&mut self, base: std::sync::Arc<Vec<u8>>) -> Result<usize> {
        if self.overlay.is_some() || self.change_list.is_some() {
            return Err(DomainError::Validation("this database is already supplemented".into()));
        }
        // An NHL 07 / NHL 08 roster save records changes to the game's database: show the
        // database with those changes applied.
        if crate::change_list_overlay::is_change_list(&base, &self.db_bytes, &self.meta) {
            let (composite, cl) =
                crate::change_list_overlay::compose(&self.db_bytes, base, &self.meta)?;
            let read_only = TdbFile::parse(&composite)?.tables.len() - cl.editable_tables();
            self.db_bytes = composite;
            self.change_list = Some(cl);
            return Ok(read_only);
        }
        match crate::table_overlay::compose(&self.db_bytes, base)? {
            Some((composite, overlay)) => {
                let added = overlay.frozen.len();
                self.db_bytes = composite;
                self.overlay = Some(overlay);
                Ok(added)
            }
            None => Ok(0),
        }
    }

    /// The read-only tables supplied from the game's database (raw ids), when supplemented.
    pub fn frozen_tables(&self) -> Option<&std::collections::BTreeSet<String>> {
        match (&self.overlay, &self.change_list) {
            (Some(o), _) => Some(&o.frozen),
            (_, Some(cl)) => Some(cl.frozen()),
            _ => None,
        }
    }

    /// `true` when this project edits a roster change list ([`crate::change_list_overlay`]).
    pub fn is_change_list(&self) -> bool {
        self.change_list.is_some()
    }

    /// Refuse a working database the file cannot record (a change-list save): the same build a
    /// save performs, without keeping its result.
    pub(crate) fn check_recordable(&self, working: &[u8]) -> Result<()> {
        match &self.change_list {
            Some(cl) => cl.change_list_for(working, &self.meta).map(|_| ()),
            None => Ok(()),
        }
    }

    /// How many leading tables of the working database belong to the file itself.
    pub fn own_table_count(&self) -> Option<usize> {
        match (&self.overlay, &self.change_list) {
            (Some(o), _) => Some(o.save_tables),
            (_, Some(cl)) => Some(cl.editable_tables()),
            _ => None,
        }
    }

    /// The working-database form of an inner database read from this project's file: the same
    /// composite a load builds when supplemented, else the bytes unchanged.
    pub(crate) fn compose_inner(&self, inner: Vec<u8>) -> Result<Vec<u8>> {
        if let Some(cl) = &self.change_list {
            return cl.recompose(&inner, &self.meta);
        }
        let Some(o) = &self.overlay else {
            return Ok(inner);
        };
        match crate::table_overlay::compose(&inner, o.base.clone())? {
            Some((composite, again)) if again.frozen == o.frozen && again.save_tables == o.save_tables => Ok(composite),
            _ => Err(DomainError::Validation(
                "the file's tables changed shape since it was opened; reopen it".into(),
            )),
        }
    }

    /// Build a project from already-acquired file bytes and registries. Detects raw-db vs. container,
    /// extracts the inner TDB, and verifies the byte order matches the declared platform.
    pub fn from_parts(
        file_bytes: Vec<u8>,
        platform: Platform,
        role: ProjectRole,
        meta: FieldMetaIndex,
        relations: RelationRegistry,
        enums: EnumRegistry,
    ) -> Result<Self> {
        let (source, db_bytes) = detect_source(file_bytes)?;
        let db = TdbFile::parse(&db_bytes)?;
        if db.endian != platform.endian() {
            return Err(DomainError::EndianMismatch {
                platform: platform.as_str(),
                endian: db.endian,
            });
        }
        // Title-scoped vocabularies resolve for the project's title only; until `set_game_id`
        // names one, only the unscoped vocabularies apply (no labels are borrowed on a guess).
        let mut enums = enums;
        enums.set_scope(None, platform.as_str());
        Ok(Project {
            platform,
            role,
            source,
            db_bytes,
            meta,
            relations,
            enums,
            loaded_identity: None,
            leagues: crate::leagues::LeagueVocabularies::default(),
            game_id: None,
            settings: crate::settings_catalogue::SettingsDefs::default(),
            league_imagery: crate::league_imagery::LeagueImageryDefs::default(),
            glyphs: crate::font_glyphs::GlyphCoverageDefs::default(),
            schedule_stock: crate::schedule_stock::StockPartitionDefs::default(),
            overlay: None,
            change_list: None,
        })
    }

    /// The shipped-font glyph coverage loaded from `defs/fonts/glyph-coverage.toml`.
    pub fn glyph_coverage(&self) -> &crate::font_glyphs::GlyphCoverageDefs {
        &self.glyphs
    }

    /// The league vocabularies loaded from `defs/leagues`.
    pub fn leagues(&self) -> &crate::leagues::LeagueVocabularies {
        &self.leagues
    }

    /// Replace the league vocabularies (projects built from parts, tests).
    pub fn set_leagues(&mut self, leagues: crate::leagues::LeagueVocabularies) {
        self.leagues = leagues;
    }

    /// The settings-catalogue support data loaded from `defs/settings`.
    pub fn settings_defs(&self) -> &crate::settings_catalogue::SettingsDefs {
        &self.settings
    }

    /// Replace the settings support data (projects built from parts, tests).
    pub fn set_settings_defs(&mut self, defs: crate::settings_catalogue::SettingsDefs) {
        self.settings = defs;
    }

    /// The league presentation imagery support data loaded from `defs/league_imagery.toml`.
    pub fn league_imagery_defs(&self) -> &crate::league_imagery::LeagueImageryDefs {
        &self.league_imagery
    }

    /// The studio game id this project was opened as, when known.
    pub fn game_id(&self) -> Option<&str> {
        self.game_id.as_deref()
    }

    pub fn set_game_id(&mut self, game_id: Option<String>) {
        self.enums.set_scope(game_id.as_deref(), self.platform.as_str());
        self.game_id = game_id;
    }

    /// The inner TDB bytes (already extracted from any container).
    pub fn db_bytes(&self) -> &[u8] {
        &self.db_bytes
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn meta(&self) -> &FieldMetaIndex {
        &self.meta
    }

    pub fn relations(&self) -> &RelationRegistry {
        &self.relations
    }

    pub fn enums(&self) -> &EnumRegistry {
        &self.enums
    }

    /// Wrap (or pass through) edited inner-db bytes into the project's on-disk form. For a raw db this
    /// is the db itself; for a container it repacks through `tdb-savedata` (recomputing the container
    /// checksums). The inner db must already be CRC-valid (the edit session commits before saving).
    pub fn package(&self, inner_db: &[u8]) -> Result<Vec<u8>> {
        // A change list is rebuilt from the composite into its fixed-size container.
        if let Some(cl) = &self.change_list {
            let list = cl.strip(inner_db, &self.meta)?;
            return match &self.source {
                Source::RawDb => Ok(list),
                Source::Container { original } => {
                    crate::change_list_overlay::pack_container(original, &list)
                }
                Source::Ps4Save { .. } => Err(DomainError::Validation(
                    "roster change lists are not stored in PS4 saves".into(),
                )),
            };
        }
        // A supplemented database writes back only the file's own tables.
        let stripped;
        let inner_db = match &self.overlay {
            Some(o) => {
                stripped = crate::table_overlay::strip(inner_db, o)?;
                &stripped[..]
            }
            None => inner_db,
        };
        match &self.source {
            Source::RawDb => Ok(inner_db.to_vec()),
            Source::Container { original } => {
                let container = SaveContainer::parse(original)?;
                Ok(container.repack(inner_db)?)
            }
            Source::Ps4Save { original, trailer, original_db_len } => {
                // The opaque suffix belongs to the DATA record, not the TDB CRC scope.
                // Its offset semantics are unknown, so do not move it by growing the DB.
                if !trailer.is_empty() && inner_db.len() != *original_db_len {
                    return Err(DomainError::Validation("Cannot resize a PS4 roster with an opaque trailer.".into()));
                }
                let mut payload = inner_db.to_vec();
                payload.extend_from_slice(trailer);
                Ok(tdb_savedata::ps4_repack_roster(original, &payload)?)
            }
        }
    }
}

/// What a re-parse of packaged bytes established (S2-B adapter): the inner TDB's byte length and
/// table count, after the same detection and endian check a load performs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReparsedDb {
    pub inner_len: usize,
    pub table_count: usize,
}

impl Project {
    /// Re-parse packaged on-disk bytes (as [`Project::package`] produces them) exactly as a load
    /// would — source detection, inner TDB parse, endian check — without building registries. Used
    /// to validate a serialized save/DB artifact before it enters a transaction.
    pub fn reparse_packaged(file_bytes: Vec<u8>, platform: Platform) -> Result<ReparsedDb> {
        let (_, inner) = detect_source(file_bytes)?;
        let db = TdbFile::parse(&inner)?;
        if db.endian != platform.endian() {
            return Err(DomainError::EndianMismatch {
                platform: platform.as_str(),
                endian: db.endian,
            });
        }
        Ok(ReparsedDb {
            inner_len: inner.len(),
            table_count: db.tables.len(),
        })
    }
}

/// [`detect_source`] for a caller outside this module that needs the inner bytes before deciding
/// whether to adopt the file (the session's post-save verification).
pub(crate) fn detect_source_for_check(file_bytes: Vec<u8>) -> Result<(Source, Vec<u8>)> {
    detect_source(file_bytes)
}

/// Classify the file: a parseable TDB is a raw db; otherwise try a save container and extract its
/// inner db; otherwise it is not a database we recognise.
fn detect_source(file_bytes: Vec<u8>) -> Result<(Source, Vec<u8>)> {
    if TdbFile::parse(&file_bytes).is_ok() {
        return Ok((Source::RawDb, file_bytes));
    }
    if let Ok(container) = SaveContainer::parse(&file_bytes) {
        let inner = container.extract_db()?;
        return Ok((
            Source::Container {
                original: file_bytes,
            },
            inner,
        ));
    }
    // A PS4 save DATA stream: the live roster uses chunkzip (NHL15) or LZ4 (later titles).
    if tdb_savedata::is_ps4_save(&file_bytes) {
        let mut inner = tdb_savedata::ps4_extract_roster(&file_bytes)?;
        let declared = TdbFile::parse(&inner)?.db_size as usize;
        if declared > inner.len() || declared < 0x1c {
            return Err(DomainError::Validation("Invalid PS4 roster database length.".into()));
        }
        let trailer = inner.split_off(declared);
        TdbFile::parse(&inner)?;
        return Ok((
            Source::Ps4Save {
                original: file_bytes,
                trailer,
                original_db_len: declared,
            },
            inner,
        ));
    }
    Err(DomainError::NotADatabase)
}

fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|source| DomainError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn read_to_string(path: &Path) -> Result<String> {
    let bytes = read(path)?;
    // Some shipped discs keep the sibling metadata XML RefPack-compressed.
    // Decode in memory so opening an extracted disc never rewrites its files.
    let bytes = if tdb_archive_codec::is_refpack(&bytes) {
        tdb_archive_codec::refpack_decompress(&bytes)
            .map_err(|e| DomainError::Validation(format!("{}: metadata RefPack: {e}", path.display())))?
    } else {
        bytes
    };
    String::from_utf8(bytes).map_err(|error| DomainError::Io {
        path: path.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, error),
    })
}
