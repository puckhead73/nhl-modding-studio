//! Game-to-game headmesh conversion engine (Appearance Studio, pillar 1).
//!
//! Re-derives the proven NHL24 → NHL Legacy head-import process as the first entry of a
//! generalized per-(source, target) conversion matrix. Two pipelines share one spine:
//!
//! * **Path A — authored weights** (source ships per-vertex weights against a nameable
//!   skeleton, e.g. NHL24 Frostbite exports): geometry carries through, weights are
//!   remapped onto the target rig's bone palette.
//! * **Path B — donor morph** (source is bare geometry): a target-native donor head is
//!   shrinkwrapped onto the source shape in headless Blender (`blender-job` crate); the
//!   donor's topology, UVs, and animated weights survive.
//!
//! Stage spine: extract → frame-adapt → (remap | morph) → pack → texture → QA →
//! install-plan (pure) → install-apply (writes). Every stage is capability-gated by the
//! matrix (`defs/appearance/headmesh-matrix.toml`): a pair that has not been verified for
//! an operation refuses it rather than guessing — same proven-first discipline as the DB
//! operation matrix.
//!
//! This crate is engine-only: no Tauri, no UI. The `headmesh` CLI binary and the
//! `studio-domain`/`studio-api` appearance surfaces call the same library functions.

pub mod cc5;
pub mod census;
pub mod convert;
pub mod extract;
pub mod facial;
pub mod frames;
pub mod glb;
pub mod install;
pub mod ir;
pub mod library;
pub mod matrix;
pub mod morph;
pub mod nhl19_fit;
pub mod pack_mcd;
pub mod pack_rw;
pub mod palette;
pub mod placement;
pub mod qa;
pub mod remap;
pub mod sidecar;
pub mod texture;
pub mod transfer;
pub mod uvfp;

pub use ir::{HeadIr, SubmeshRole, UvConvention};
pub use matrix::{ConversionMatrix, GameKey, PairStatus};
