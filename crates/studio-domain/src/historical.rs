//! Historical (FHM corpus) read access — the domain's window onto the read-only historical source
//! database. Unlike the EA-DB path there is no edit session: the corpus is a *source* you browse and
//! import *from*, never edit. So this module simply surfaces the read-only [`FhmCorpus`] store and
//! its shared row model as the domain-layer type, keeping `studio-api` from reaching past the domain
//! into the engine crates.
//!
//! Name resolution (team/league ids → names) already happens inside the store at build/query time
//! (Human-Readable-Surfaces rule), so the model rows that cross this boundary are already display-ready.

pub use tdb_fhm::model::{
    AwardRow, CareerStatRow, ContractRow, DraftRow, LeagueOption, PersonDetail, PersonSummary,
    PlayerSeasonRow, SeasonOption, TeamAffiliate, TeamContractLine, TeamFranchiseDetail,
    TeamIndexEntry, TeamRosterEntry, TeamRosterMember, TeamSeasonBlock, TeamSeasonDetail,
    TeamSeasonSummary,
};
pub use tdb_fhm_store::{FhmCorpus, FhmError};
