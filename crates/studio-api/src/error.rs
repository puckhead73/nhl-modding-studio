//! The API-layer error: domain failures plus the registry/marshalling concerns that only exist at
//! the command surface (unknown project id, bad enum string, io while saving).

use std::fmt;

use studio_domain::DomainError;

#[derive(Debug)]
pub enum ApiError {
    Domain(DomainError),
    /// No session is registered under this project id.
    UnknownProject(u32),
    /// A string argument (platform/role) was not a recognised value.
    BadEnum { what: &'static str, value: String },
    /// Filesystem error while saving a project.
    Io { path: String, source: std::io::Error },
    /// Onboarding couldn't resolve the files a workspace needs from the chosen root folder.
    Workspace(String),
    /// No FHM historical corpus has been opened (call `open_corpus` first).
    CorpusNotOpen,
    /// A read against the FHM corpus failed (query error or schema mismatch).
    Corpus(studio_domain::FhmError),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Domain(e) => write!(f, "{e}"),
            ApiError::UnknownProject(id) => write!(f, "no project with id {id}"),
            ApiError::BadEnum { what, value } => write!(f, "unknown {what}: {value:?}"),
            ApiError::Io { path, source } => write!(f, "io error on {path:?}: {source}"),
            ApiError::Workspace(msg) => write!(f, "{msg}"),
            ApiError::CorpusNotOpen => write!(
                f,
                "the historical (FHM) corpus is not open — open a data pack first"
            ),
            ApiError::Corpus(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<studio_domain::FhmError> for ApiError {
    fn from(e: studio_domain::FhmError) -> Self {
        ApiError::Corpus(e)
    }
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        ApiError::Domain(e)
    }
}
