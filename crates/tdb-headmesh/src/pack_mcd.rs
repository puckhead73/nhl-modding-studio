//! Pack a fitted head back into a PS4 RSF `.mcd` container.
//!
//! This is the **donor-preserving** path and the only one this module offers: the donor
//! container supplies topology, vertex order, indices, UVs, blend data, names, bindings
//! and every opaque byte, and the caller supplies replacement *positions* for one
//! submesh at exactly the donor's vertex count. Nothing else is substituted.
//!
//! Container reading, refusal and re-serialization live in [`tdb_rx2::mcd_write`]; this
//! module is the head-pipeline-facing adapter over it. Source-topology replacement and
//! weight transfer are deliberately **not** implemented — they are deferred research,
//! not a fallback this function silently takes.
//!
//! **The donor must be a fingerprinted profile.** Both entry points go through
//! [`McdContainer::parse_with_profile`], which selects a measured
//! [`DonorProfile`](tdb_rx2::mcd_write::DonorProfile) by content hash. A vertex-count
//! match cannot establish vertex *order* or protected-region identity, so an arbitrary
//! parseable `.mcd` — even one with the right counts — is refused rather than edited
//! against an unverified layout.

use tdb_rx2::mcd_write::{McdContainer, McdWriteError};

/// Donor submesh a head fit targets.
pub const HEAD_SUBMESH: &str = "geo_headShape";

#[derive(Debug, thiserror::Error)]
pub enum PackMcdError {
    #[error("donor container refused: {0}")]
    Donor(#[from] McdWriteError),
    #[error("donor has no {0:?} submesh")]
    NoSuchSubmesh(String),
    #[error("donor submesh {submesh:?} carries no position stream")]
    NoPositionStream { submesh: String },
    #[error(
        "fitted positions must match the donor exactly: {submesh:?} has {donor} vertices, got \
         {given} — this path never changes vertex count, order or topology"
    )]
    VertexCountMismatch {
        submesh: String,
        donor: usize,
        given: usize,
    },
    #[error(
        "PS4 .mcd source-topology substitution is not implemented (deferred research); use the \
         donor-preserving positional path"
    )]
    TopologySubstitutionUnsupported,
}

/// How many donor vertices a submesh expects, so a caller can size a fit before running it.
pub fn donor_vertex_count(donor_mcd: &[u8], submesh: &str) -> Result<usize, PackMcdError> {
    let c = McdContainer::parse_with_profile(donor_mcd)?;
    let sub = c
        .submesh_index(submesh)
        .ok_or_else(|| PackMcdError::NoSuchSubmesh(submesh.to_string()))?;
    let stream = c
        .position_stream_of(sub)
        .ok_or_else(|| PackMcdError::NoPositionStream {
            submesh: submesh.to_string(),
        })?;
    Ok(c.streams[stream].positions().len())
}

/// Substitute one submesh's positions in `donor_mcd` and re-emit the container.
///
/// `donor_mcd` must hash to a known [`DonorProfile`](tdb_rx2::mcd_write::DonorProfile),
/// and `positions` must have exactly that donor submesh's vertex count, in donor vertex
/// order. The submesh's tail AABB is recomputed from the result; every other byte of the
/// container is preserved. Passing the donor's own positions back reproduces the input
/// byte-for-byte.
///
/// The length check is a guard, not a proof: it cannot establish that `positions` are in
/// donor vertex order. Callers producing a fit must carry the donor fingerprint, vertex
/// ids and protected masks alongside it.
pub fn pack_positions_into_donor(
    donor_mcd: &[u8],
    submesh: &str,
    positions: &[[f32; 3]],
) -> Result<Vec<u8>, PackMcdError> {
    let mut c = McdContainer::parse_with_profile(donor_mcd)?;
    let sub = c
        .submesh_index(submesh)
        .ok_or_else(|| PackMcdError::NoSuchSubmesh(submesh.to_string()))?;
    let stream = c
        .position_stream_of(sub)
        .ok_or_else(|| PackMcdError::NoPositionStream {
            submesh: submesh.to_string(),
        })?;
    let donor = c.streams[stream].positions().len();
    if positions.len() != donor {
        return Err(PackMcdError::VertexCountMismatch {
            submesh: submesh.to_string(),
            donor,
            given: positions.len(),
        });
    }
    c.set_positions(stream, positions)?;
    c.refresh_bounds(sub);
    Ok(c.serialize())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Refusals this adapter owns are reported as its own errors, not as a bare
    /// container error, so a caller can tell "wrong donor" from "wrong fit".
    #[test]
    fn a_non_container_is_refused_as_a_donor_error() {
        let err = pack_positions_into_donor(b"not an rsf", HEAD_SUBMESH, &[]).unwrap_err();
        assert!(matches!(err, PackMcdError::Donor(_)), "got {err:?}");
    }

    #[test]
    fn donor_vertex_count_refuses_a_non_container() {
        assert!(donor_vertex_count(b"nope", HEAD_SUBMESH).is_err());
    }

    /// The gate is the donor fingerprint, so anything unrecognised is refused before any
    /// vertex-count reasoning happens.
    #[test]
    fn an_unfingerprinted_donor_is_refused_before_any_fit_is_considered() {
        let mut fake = tdb_rx2::mcd::MAGIC_RSF.to_vec();
        fake.extend_from_slice(&[0u8; 4096]);
        let err = pack_positions_into_donor(&fake, HEAD_SUBMESH, &[[0.0; 3]; 3157]).unwrap_err();
        match err {
            PackMcdError::Donor(tdb_rx2::mcd_write::McdWriteError::UnknownDonor { .. }) => {}
            other => panic!("expected UnknownDonor, got {other:?}"),
        }
    }
}
