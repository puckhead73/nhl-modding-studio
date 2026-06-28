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
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Domain(e) => write!(f, "{e}"),
            ApiError::UnknownProject(id) => write!(f, "no project with id {id}"),
            ApiError::BadEnum { what, value } => write!(f, "unknown {what}: {value:?}"),
            ApiError::Io { path, source } => write!(f, "io error on {path:?}: {source}"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        ApiError::Domain(e)
    }
}
