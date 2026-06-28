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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        let meta_xml = meta_xml.as_ref();
        let meta_text = read_to_string(meta_xml)?;

        let defs_root = defs_root.as_ref();
        let relations = RelationRegistry::from_dir(defs_root.join("relationships"))?;
        let enums = EnumRegistry::from_dir(defs_root.join("enums"))?;
        let meta = FieldMetaIndex::from_xml(&meta_text)?;

        Self::from_parts(file_bytes, platform, role, meta, relations, enums)
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
        Ok(Project {
            platform,
            role,
            source,
            db_bytes,
            meta,
            relations,
            enums,
        })
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
        match &self.source {
            Source::RawDb => Ok(inner_db.to_vec()),
            Source::Container { original } => {
                let container = SaveContainer::parse(original)?;
                Ok(container.repack(inner_db)?)
            }
        }
    }
}

/// Classify the file: a parseable TDB is a raw db; otherwise try a save container and extract its
/// inner db; otherwise it is not a database we recognise.
fn detect_source(file_bytes: Vec<u8>) -> Result<(Source, Vec<u8>)> {
    if TdbFile::parse(&file_bytes).is_ok() {
        return Ok((Source::RawDb, file_bytes));
    }
    if let Ok(container) = SaveContainer::parse(&file_bytes) {
        let inner = container.extract_db()?;
        return Ok((Source::Container { original: file_bytes }, inner));
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
    fs::read_to_string(path).map_err(|source| DomainError::Io {
        path: path.display().to_string(),
        source,
    })
}
