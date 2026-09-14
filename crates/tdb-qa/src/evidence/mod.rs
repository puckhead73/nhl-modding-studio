//! QA evidence identity, merge and publication —
//! [docs/plans/qa-evidence-contract.md](../../../../docs/plans/qa-evidence-contract.md) at **R2**.
//!
//! The contract separates four layers and this module keeps them separate structurally:
//!
//! | Layer | Contents | Written by |
//! |---|---|---|
//! | **A** — observations | what an automated case actually did, in an immutable run | [`observe`] |
//! | **B** — manual records | durable human verification | humans, and an approved [`migrate`] install |
//! | **L** — legacy claims | pre-contract status labels, migrated verbatim; never qualify | [`migrate`] |
//! | **C** — derived summaries | what the docs and guards read | [`merge`] + [`publish`] |
//!
//! **R-SEP.** Nothing in this crate writes under the manual root except
//! [`migrate::install`](migrate::install), which refuses without a matching approval digest. The
//! merge itself ([`merge::merge`]) is a pure function: it reads no clock, no filesystem and no
//! timestamp field, so every acceptance case runs over synthetic fixtures.
//!
//! Nothing here promotes a support status, and no value produced by [`testkit`] is evidence about
//! any real target.

pub mod csv;
pub mod emit;
pub mod journal;
pub mod key;
pub mod merge;
pub mod migrate;
pub mod model;
pub mod observe;
pub mod paths;
pub mod publish;
pub mod scopes;
pub mod store;
pub mod testkit;
pub mod validate;
pub mod vocab;

pub use key::{Conversion, CoverageKey, KeyError};
pub use merge::{AutomatedAxis, Freshness, ManualAxis, Merged, Qualification, RollUp, SummaryRow};
pub use model::{
    EvidenceSet, Facet, Journal, ManualRecord, Observation, Outcome, Run, RunState,
    UnmappedObservation,
};
pub use validate::Violation;
pub use vocab::{Applicability, OperationClass};
