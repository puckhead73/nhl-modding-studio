//! Local fingerprint/layout gate over the read-only Studio dependency. No new Studio profiles.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tdb_rx2::mcd_write::McdContainer;
pub fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
pub fn layout(c: &McdContainer) -> Value {
    json!({"streams":c.streams.iter().map(|s|json!({"chunk":s.chunk_off,"data":s.data_off,"count":s.count,"stride":s.stride,"position_offset":s.pos_off})).collect::<Vec<_>>(),
        "submeshes":c.submeshes.iter().enumerate().map(|(i,s)|json!({"name":s.name,"mesh_offset":s.mesh_off,"triangles":s.triangles,"format":s.format,"streams":s.streams,"index_chunk":s.index_chunk,"shared_indices":s.shares_index_buffer,"topology_sha256":hash(&serde_json::to_vec(&c.triangles_of(i)).unwrap())})).collect::<Vec<_>>(),
        "writable_spans":c.writable_spans(),"unclaimed_indices":c.unclaimed_index_chunks,"region_counts":c.region_counts()})
}
pub fn parse_pinned(d: &[u8], p: &Value) -> Result<McdContainer, String> {
    if p["schema"] != "nhl-mcd-profile/1" {
        return Err("unsupported profile schema".into());
    }
    if p["donor"]["sha256"] != hash(d) || p["donor"]["bytes"] != d.len() {
        return Err("donor fingerprint mismatch".into());
    }
    let c = McdContainer::parse(d).map_err(|e| e.to_string())?;
    if p["layout"] != layout(&c) {
        return Err("donor layout mismatch".into());
    }
    if c.serialize() != d {
        return Err("donor unchanged round-trip mismatch".into());
    }
    Ok(c)
}

/// Permit positions plus bounds only. Other vertex attributes remain protected even
/// though the underlying library's wider writable_spans covers whole streams.
pub fn position_bounds_mask(c: &McdContainer) -> Vec<bool> {
    let mut allowed = vec![false; c.serialize().len()];
    for (lo, hi) in c.writable_spans() {
        if !c
            .streams
            .iter()
            .any(|s| lo == s.data_off && hi == s.data_off + s.count * s.stride)
        {
            allowed[lo..hi].fill(true);
        }
    }
    for s in &c.streams {
        if let Some(po) = s.pos_off {
            for i in 0..s.count {
                let lo = s.data_off + i * s.stride + po;
                allowed[lo..lo + 12].fill(true);
            }
        }
    }
    allowed
}

#[cfg(test)]
#[path = "../../tests/adapters/mcd_profile_tests.rs"]
mod tests;
