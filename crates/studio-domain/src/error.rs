//! One error type for the whole domain layer, wrapping each engine crate's error.

use std::fmt;

/// Anything that can go wrong loading a project or driving an edit session.
#[derive(Debug)]
pub enum DomainError {
    /// Filesystem error reading/writing a project file (path included for context).
    Io { path: String, source: std::io::Error },
    /// The bytes were neither a parseable TDB nor a recognised save container.
    NotADatabase,
    /// The on-disk byte order contradicts the declared platform (e.g. `Ps4` but big-endian).
    EndianMismatch { platform: &'static str, endian: tdb_core::Endian },
    /// The inner TDB failed to parse.
    Tdb(tdb_core::TdbError),
    /// The metadata XML failed to parse.
    Meta(tdb_defs::MetaError),
    /// An enum def failed to load.
    Enums(tdb_enums::EnumError),
    /// A relationship def failed to load.
    Relations(tdb_relations::RelationError),
    /// A save-container codec error (extract/repack).
    Save(tdb_savedata::SaveError),
    /// An edit operation failed (validation, audit rollback, allocation, …).
    Edit(tdb_edit::EditError),
    /// A multi-file (roster ↔ nhlng) operation could not keep the two files in sync; both have been
    /// rolled back to their pre-op state.
    Sync(String),
    /// Big-archive extraction failed (no archives found, or every archive failed to parse).
    Extract(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::Io { path, source } => write!(f, "io error on {path:?}: {source}"),
            DomainError::NotADatabase => {
                write!(f, "not a TDB database or recognised save container")
            }
            DomainError::EndianMismatch { platform, endian } => write!(
                f,
                "platform {platform} expects a different byte order than the file's {endian:?}"
            ),
            DomainError::Tdb(e) => write!(f, "{e}"),
            DomainError::Meta(e) => write!(f, "{e}"),
            DomainError::Enums(e) => write!(f, "{e}"),
            DomainError::Relations(e) => write!(f, "{e}"),
            DomainError::Save(e) => write!(f, "{e}"),
            DomainError::Edit(e) => write!(f, "{e}"),
            DomainError::Sync(msg) => write!(f, "{msg}"),
            DomainError::Extract(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for DomainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DomainError::Io { source, .. } => Some(source),
            DomainError::Tdb(e) => Some(e),
            DomainError::Meta(e) => Some(e),
            DomainError::Enums(e) => Some(e),
            DomainError::Relations(e) => Some(e),
            DomainError::Save(e) => Some(e),
            DomainError::Edit(e) => Some(e),
            _ => None,
        }
    }
}

macro_rules! from_impl {
    ($variant:ident, $ty:ty) => {
        impl From<$ty> for DomainError {
            fn from(e: $ty) -> Self {
                DomainError::$variant(e)
            }
        }
    };
}

from_impl!(Tdb, tdb_core::TdbError);
from_impl!(Meta, tdb_defs::MetaError);
from_impl!(Enums, tdb_enums::EnumError);
from_impl!(Relations, tdb_relations::RelationError);
from_impl!(Save, tdb_savedata::SaveError);
from_impl!(Edit, tdb_edit::EditError);
