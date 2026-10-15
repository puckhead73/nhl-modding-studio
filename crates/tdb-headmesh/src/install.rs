//! Install planning — pure id-scheme rules + planned writes. Applying a plan (actual
//! file writes, `.bak`, DB field staging) is the studio-domain layer's job; this module
//! never touches the filesystem.

use serde::Serialize;

use crate::matrix::IdScheme;

/// The generic-head pool floor on Legacy / NHL 15 — ids at or above this are shared
/// pool heads, so a bespoke install must stay below it.
pub const POOL_HEAD_MIN: u32 = 60_000;
/// `ubPc.pPaD` is a 15-bit field on NHL 11–14.
pub const PPAD_15BIT_MAX: u32 = 32_767;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum InstallError {
    #[error("head id {id} exceeds the 15-bit pPaD ceiling ({PPAD_15BIT_MAX}) on this title")]
    IdOver15Bit { id: u32 },
    #[error("head id {id} is inside the generic pool range (≥ {POOL_HEAD_MIN}) — bespoke installs must use an id below it")]
    IdInPoolRange { id: u32 },
    #[error("install planning for this target's id scheme is not audited yet ({0})")]
    SchemeUnaudited(&'static str),
}

/// One file the install will write, relative to the target's rendering root.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlannedWrite {
    pub rel_path: String,
    pub kind: PlannedWriteKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedWriteKind {
    Mesh,
    TextureLibrary,
}

/// A pure install plan: files to place + the DB staging the caller must perform.
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    pub head_id: u32,
    pub writes: Vec<PlannedWrite>,
    /// Reminder of the bespoke-head invariant the DB edit must establish:
    /// `ubPc.pPaD == ubPc.lOnr == head_id`.
    pub requires_ppad_eq_lonr: bool,
}

/// Validate a bespoke head id for a target scheme and produce the planned writes.
pub fn plan_rw_install(scheme: IdScheme, head_id: u32) -> Result<InstallPlan, InstallError> {
    match scheme {
        IdScheme::RwPpad => {
            if head_id >= POOL_HEAD_MIN {
                return Err(InstallError::IdInPoolRange { id: head_id });
            }
        }
        IdScheme::RwPpad15Bit => {
            if head_id > PPAD_15BIT_MAX {
                return Err(InstallError::IdOver15Bit { id: head_id });
            }
        }
        IdScheme::Ps4Slug => return Err(InstallError::SchemeUnaudited("ps4-slug (HM-S3d)")),
        IdScheme::Unaudited => return Err(InstallError::SchemeUnaudited("pre-NHL11 layout")),
    }
    Ok(InstallPlan {
        head_id,
        writes: vec![
            PlannedWrite {
                rel_path: format!("player/player_{head_id}.rx2"),
                kind: PlannedWriteKind::Mesh,
            },
            PlannedWrite {
                rel_path: format!("player/texlib_{head_id}.rx2"),
                kind: PlannedWriteKind::TextureLibrary,
            },
        ],
        requires_ppad_eq_lonr: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rw_ppad_rules() {
        let plan = plan_rw_install(IdScheme::RwPpad, 30_101).unwrap();
        assert_eq!(plan.writes[0].rel_path, "player/player_30101.rx2");
        assert_eq!(plan.writes[1].rel_path, "player/texlib_30101.rx2");
        assert!(plan.requires_ppad_eq_lonr);
        assert_eq!(
            plan_rw_install(IdScheme::RwPpad, 60_000).unwrap_err(),
            InstallError::IdInPoolRange { id: 60_000 }
        );
    }

    #[test]
    fn fifteen_bit_rules() {
        assert!(plan_rw_install(IdScheme::RwPpad15Bit, 32_767).is_ok());
        assert_eq!(
            plan_rw_install(IdScheme::RwPpad15Bit, 32_768).unwrap_err(),
            InstallError::IdOver15Bit { id: 32_768 }
        );
    }

    #[test]
    fn unaudited_schemes_refuse() {
        assert!(matches!(
            plan_rw_install(IdScheme::Ps4Slug, 1).unwrap_err(),
            InstallError::SchemeUnaudited(_)
        ));
        assert!(matches!(
            plan_rw_install(IdScheme::Unaudited, 1).unwrap_err(),
            InstallError::SchemeUnaudited(_)
        ));
    }
}
