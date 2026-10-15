//! Headless-Blender job harness (ROADMAP 6.2).
//!
//! Contract: `blender --background --factory-startup --python <script> -- <request.json>`.
//! The script's **last act** is writing `response.json` next to the request; stdout and
//! stderr are captured to `blender.log`. A nonzero exit, a timeout, or a missing/invalid
//! response file is a *surfaced job failure with the log tail attached* — never a silent
//! one (the 6.2 gate).
//!
//! Determinism: results depend on the Blender build, so the version is pinned
//! ([`bundle::PINNED_VERSION`]). Blender is **not** bundled into the installer
//! (~hundreds of MB, and Path A never needs it): it is downloaded on demand into a
//! managed directory, or the user points at their own install (warned when the version
//! differs from the pin).
//!
//! This crate is context-free: no app paths, no Tauri. Callers pass every location in.

pub mod bundle;
pub mod invoke;

pub use bundle::{BlenderLocation, LocateError};
pub use invoke::{JobError, JobOutcome, JobSpec};

/// The generalized donor-wrap script (Path B), embedded so the binary is self-contained
/// and the script version is pinned to the build.
pub const WRAP_HEAD_SCRIPT: &str = include_str!("../scripts/wrap_head.py");
