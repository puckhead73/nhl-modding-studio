//! Per-(source, target) conversion capability matrix.
//!
//! Loaded from `defs/appearance/headmesh-matrix.toml`. Every conversion request is
//! validated against a `[[pair]]` row; a pair that is absent or `unproven` is refused
//! (CLI-only `allow_unproven` escape hatch for spike work). Promotion to
//! `roundtrip_verified` / `game_verified` requires dated evidence — an audit link —
//! mirroring the DB operation-matrix discipline. NHL24→Legacy is simply the first row
//! to earn `game_verified`; nothing in the engine special-cases it.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::Path;

/// A game or corpus participating in the matrix.
///
/// Installed titles are keyed `title-platform` (`nhllegacy-xbox360`, `nhl16-ps4`);
/// source-only corpora are keyed by name alone (`nhl24`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GameKey(pub String);

impl GameKey {
    pub fn title(&self) -> &str {
        self.0.split('-').next().unwrap_or(&self.0)
    }

    /// Platform component, when the key names an installed title.
    pub fn platform(&self) -> Option<&str> {
        let mut parts = self.0.splitn(2, '-');
        parts.next();
        parts.next()
    }
}

impl fmt::Display for GameKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Verification status of a pair. Order matters: later variants imply the earlier gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairStatus {
    /// No verified conversion; the engine refuses this pair.
    Unproven,
    /// Convert + install round-trips verified against fixtures; no in-game check yet.
    RoundtripVerified,
    /// A converted head has been seen rendering correctly in the target game (manual gate).
    GameVerified,
}

/// Which pipeline(s) a pair supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PairPath {
    /// Path A: source authored weights remapped onto the target rig.
    AuthoredWeights,
    /// Path B: target donor shrinkwrapped onto source geometry (Blender).
    DonorMorph,
    /// Path C: source geometry + UVs carry through, but the skin weights are transferred
    /// off the target donor's own rigged surface by proximity. For sources that ship a
    /// mesh with no usable articulation (Reallusion CC5 head exports), where Path A has
    /// no rig to retarget and Path B would discard the source topology.
    DonorWeights,
    Both,
}

/// How the target keys installed heads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdScheme {
    /// RW `player_<pPaD>.rx2` + `texlib_<pPaD>.rx2`; bespoke ⇔ `ubPc.pPaD == ubPc.lOnr`;
    /// generic pool parked at ids ≥ 60000 (Legacy / NHL 15).
    RwPpad,
    /// Same file scheme, but `pPaD` is a 15-bit field (max 32767) and no 60000 pool
    /// exists — NHL 11–14.
    #[serde(rename = "rw-ppad-15bit")]
    RwPpad15Bit,
    /// PS4 slug-keyed `characters/player/<slug>.mcd` + per-map GNF textures. Binding of
    /// roster → slug is unaudited (HM-S3d) — install stays blocked until it is.
    Ps4Slug,
    /// Layout not yet audited (NHL 10 bespoke layout, NHL 07/08/09 loose files).
    Unaudited,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pair {
    pub source: GameKey,
    pub target: GameKey,
    pub path: PairPath,
    pub status: PairStatus,
    /// Audit link + date; must be non-empty once `status` leaves `Unproven`.
    #[serde(default)]
    pub evidence: String,
    /// Name of the frame adapter bridging source frame → shared model space
    /// (must resolve via [`crate::frames::FrameAdapter::by_name`]).
    pub frame_adapter: String,
    pub id_scheme: IdScheme,
    /// UV class of source/target head atlases per `defs/appearance/head-uv-classes.toml`;
    /// `None` until the UV spike has measured that game. Class equality gates
    /// **texture-only transplants** (a target-native texture onto source UVs or vice
    /// versa). Path A itself ships source UVs + source texture together and is proven to
    /// work across classes (the NHL24→Legacy recipe).
    #[serde(default)]
    pub uv_class_source: Option<String>,
    #[serde(default)]
    pub uv_class_target: Option<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Deserialize)]
struct MatrixFile {
    schema: String,
    #[serde(default, rename = "pair")]
    pairs: Vec<Pair>,
}

#[derive(Debug, Clone)]
pub struct ConversionMatrix {
    pub pairs: Vec<Pair>,
}

pub const MATRIX_SCHEMA: &str = "headmesh-matrix/1";

#[derive(Debug, thiserror::Error)]
pub enum MatrixError {
    #[error("failed to read matrix defs: {0}")]
    Io(#[from] std::io::Error),
    #[error("matrix defs are not valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unsupported matrix schema {found:?} (expected {MATRIX_SCHEMA:?})")]
    Schema { found: String },
    #[error("duplicate matrix pair {source_game} → {target}")]
    DuplicatePair {
        source_game: GameKey,
        target: GameKey,
    },
    #[error("pair {source_game} → {target} is {status:?} but carries no evidence link")]
    MissingEvidence {
        source_game: GameKey,
        target: GameKey,
        status: PairStatus,
    },
    #[error("pair {source_game} → {target} names unknown frame adapter {adapter:?}")]
    UnknownFrameAdapter {
        source_game: GameKey,
        target: GameKey,
        adapter: String,
    },
    #[error(
        "no verified conversion for {source_game} → {target}: {reason} \
         (see defs/appearance/headmesh-matrix.toml)"
    )]
    PairRefused {
        source_game: GameKey,
        target: GameKey,
        reason: String,
    },
}

impl ConversionMatrix {
    pub fn load(path: &Path) -> Result<ConversionMatrix, MatrixError> {
        Self::from_toml_str(&std::fs::read_to_string(path)?)
    }

    pub fn from_toml_str(text: &str) -> Result<ConversionMatrix, MatrixError> {
        let file: MatrixFile = toml::from_str(text)?;
        if file.schema != MATRIX_SCHEMA {
            return Err(MatrixError::Schema { found: file.schema });
        }
        let mut seen = HashSet::new();
        for p in &file.pairs {
            if !seen.insert((p.source.clone(), p.target.clone())) {
                return Err(MatrixError::DuplicatePair {
                    source_game: p.source.clone(),
                    target: p.target.clone(),
                });
            }
            if p.status != PairStatus::Unproven && p.evidence.trim().is_empty() {
                return Err(MatrixError::MissingEvidence {
                    source_game: p.source.clone(),
                    target: p.target.clone(),
                    status: p.status,
                });
            }
            if crate::frames::FrameAdapter::by_name(&p.frame_adapter).is_none() {
                return Err(MatrixError::UnknownFrameAdapter {
                    source_game: p.source.clone(),
                    target: p.target.clone(),
                    adapter: p.frame_adapter.clone(),
                });
            }
        }
        Ok(ConversionMatrix { pairs: file.pairs })
    }

    pub fn pair(&self, source: &GameKey, target: &GameKey) -> Option<&Pair> {
        self.pairs
            .iter()
            .find(|p| &p.source == source && &p.target == target)
    }

    /// Gate a conversion request. `allow_unproven` is the CLI-only spike escape hatch;
    /// UI surfaces must pass `false`.
    pub fn require_pair(
        &self,
        source: &GameKey,
        target: &GameKey,
        allow_unproven: bool,
    ) -> Result<&Pair, MatrixError> {
        let pair = self
            .pair(source, target)
            .ok_or_else(|| MatrixError::PairRefused {
                source_game: source.clone(),
                target: target.clone(),
                reason: "pair is not in the matrix".into(),
            })?;
        if pair.status == PairStatus::Unproven && !allow_unproven {
            return Err(MatrixError::PairRefused {
                source_game: source.clone(),
                target: target.clone(),
                reason: "pair is unproven".into(),
            });
        }
        Ok(pair)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
schema = "headmesh-matrix/1"

[[pair]]
source = "nhl24"
target = "nhllegacy-xbox360"
path = "authored-weights"
status = "unproven"
frame_adapter = "identity"
id_scheme = "rw-ppad"
"#;

    #[test]
    fn loads_and_gates() {
        let m = ConversionMatrix::from_toml_str(SAMPLE).unwrap();
        let src = GameKey("nhl24".into());
        let tgt = GameKey("nhllegacy-xbox360".into());
        assert!(m.require_pair(&src, &tgt, false).is_err());
        assert!(m.require_pair(&src, &tgt, true).is_ok());
        assert!(m
            .require_pair(&GameKey("nhl12-xbox360".into()), &tgt, true)
            .is_err());
        assert_eq!(tgt.title(), "nhllegacy");
        assert_eq!(tgt.platform(), Some("xbox360"));
        assert_eq!(src.platform(), None);
    }

    #[test]
    fn promotion_without_evidence_is_rejected() {
        let bad = SAMPLE.replace("status = \"unproven\"", "status = \"game_verified\"");
        assert!(matches!(
            ConversionMatrix::from_toml_str(&bad),
            Err(MatrixError::MissingEvidence { .. })
        ));
    }

    #[test]
    fn unknown_adapter_is_rejected() {
        let bad = SAMPLE.replace("identity", "warp-drive");
        assert!(matches!(
            ConversionMatrix::from_toml_str(&bad),
            Err(MatrixError::UnknownFrameAdapter { .. })
        ));
    }
}
