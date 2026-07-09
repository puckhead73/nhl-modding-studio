//! `tdb-fhm` — the FHM historical corpus contract: the versioned SQLite **schema**, the
//! **person-slug** identity rule that unifies a human across per-season exports, and the shared
//! **row model** structs that both the build binary (`tdb-fhm-build`) and the read layer
//! (`tdb-fhm-store`) speak.
//!
//! Why a person slug: the FHM season exports are per-season snapshots whose `PlayerId`/`TeamId`/
//! `LeagueId` are only valid within one `YYYY-YYYY.lg` folder (verified: the same human carries a
//! different id every season). The only stable cross-season identity is
//! `(last name, first name, date of birth)` — DOB is 100% populated in every era — so that triple,
//! normalized, is the corpus's person key. See [`person_slug`].

pub mod fixups;
pub mod model;
pub mod schema;
pub mod slug;

pub use fixups::known_name_fixup;
pub use schema::{DRAFT_TEAM_NAME_SQL, SCHEMA_SQL, SCHEMA_VERSION};
pub use slug::person_slug;
