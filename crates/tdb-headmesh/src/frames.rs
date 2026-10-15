//! Per-game-pair coordinate-frame adapters.
//!
//! The IR's shared model space is meters, standing pose, Y-up, face +Z. NHL24 and the
//! RW titles verifiably bind in that space already (Legacy palette bone 0 decomposes to
//! the NHL24 `Hips` model pose to 4 decimals), so their adapter is `Identity` — but the
//! adapter is *named in the matrix row* rather than implied, so any future source whose
//! frame differs (a scan, a different engine) gets an explicit, audited transform instead
//! of a silent assumption.

use crate::ir::HeadIr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameAdapter {
    /// Source already lives in the shared model space.
    Identity,
}

impl FrameAdapter {
    /// Resolve the adapter named in a matrix row. `None` = unknown name (matrix load
    /// rejects it).
    pub fn by_name(name: &str) -> Option<FrameAdapter> {
        match name {
            "identity" => Some(FrameAdapter::Identity),
            _ => None,
        }
    }

    /// Bring an extracted head into the shared model space.
    pub fn apply(&self, ir: &mut HeadIr) {
        match self {
            FrameAdapter::Identity => {
                let _ = ir;
            }
        }
    }
}
