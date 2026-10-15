//! Drift guard: the conversion matrix, the appearance defs, and the per-target
//! capability CSVs must never disagree (pattern: studio-domain/tests/operation_matrix.rs).
//!
//! What it pins:
//! * every appearance def loads and self-validates (schemas, evidence rules);
//! * every bone-override target resolves through the shipped palette naming;
//! * a matrix pair promoted beyond `unproven` requires a matching `heads` row in
//!   `docs/audits/data/assets/<title>/<platform>/capability.csv`, and `game_verified`
//!   in the matrix ⇔ `game_verified == pass` in the CSV;
//! * matrix `uv_class_*` values must exist in head-uv-classes.toml and be measured.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tdb_headmesh::matrix::{ConversionMatrix, PairStatus};
use tdb_headmesh::morph::MorphDonors;
use tdb_headmesh::palette::NamedPalette;
use tdb_headmesh::remap::BoneOverrides;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/tdb-headmesh sits two levels under the repo root")
        .to_path_buf()
}

fn defs_dir() -> PathBuf {
    repo_root().join("defs/appearance")
}

#[test]
fn appearance_defs_load_and_self_validate() {
    let defs = defs_dir();
    ConversionMatrix::load(&defs.join("headmesh-matrix.toml")).expect("matrix loads");
    NamedPalette::load(&defs.join("legacy-bone-names.toml")).expect("palette loads");
    BoneOverrides::load(&defs.join("bone-overrides.toml")).expect("overrides load");
    MorphDonors::load(&defs.join("morph-donors.toml")).expect("morph donors load");
}

#[test]
fn legacy_palette_carries_the_learned_flags() {
    let palette = NamedPalette::load(&defs_dir().join("legacy-bone-names.toml")).unwrap();
    assert_eq!(palette.len(), 122, "Legacy player palette is 122 slots");
    // The garbage slots learned from the donor-usage census.
    for slot in [5u32, 20, 49, 80] {
        assert!(
            palette.get(slot).unwrap().exclude,
            "palette slot {slot} must stay excluded (garbage matrix)"
        );
    }
    // Neck must resolve to the donor-used slot 6, never 5.
    assert_eq!(palette.name_to_palette()["Neck"], 6);
}

#[test]
fn bone_override_targets_resolve_through_the_palette() {
    let palette = NamedPalette::load(&defs_dir().join("legacy-bone-names.toml")).unwrap();
    let overrides = BoneOverrides::load(&defs_dir().join("bone-overrides.toml")).unwrap();
    let inversion = palette.name_to_palette();
    for (source, target) in &overrides.by_source {
        assert!(
            inversion.contains_key(target.as_str()),
            "override {source} → {target}: target is not a resolvable (non-excluded) palette bone"
        );
    }
}

#[test]
fn morph_donor_for_the_first_target_is_fully_measured() {
    let donors = MorphDonors::load(&defs_dir().join("morph-donors.toml")).unwrap();
    let d = donors.by_key("player_67").expect("player_67 registered");
    assert_eq!(d.target, "nhllegacy-xbox360");
    assert_eq!(d.head_vertex_count, 2623);
    assert!(d.eye_freeze_inner_m > 0.0 && d.eye_freeze_outer_m > d.eye_freeze_inner_m);
}

/// Parse a capability CSV into family → (header → value).
fn read_capability_csv(path: &Path) -> Option<HashMap<String, HashMap<String, String>>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next()?.split(',').collect();
    let mut rows = HashMap::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        // Minimal CSV split honoring quoted evidence fields.
        let mut fields: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        for ch in line.chars() {
            match ch {
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => fields.push(std::mem::take(&mut cur)),
                _ => cur.push(ch),
            }
        }
        fields.push(cur);
        let row: HashMap<String, String> = header
            .iter()
            .zip(fields.iter())
            .map(|(h, f)| (h.to_string(), f.to_string()))
            .collect();
        if let Some(family) = row.get("asset_family") {
            rows.insert(family.clone(), row);
        }
    }
    Some(rows)
}

#[test]
fn promoted_pairs_match_the_capability_csvs() {
    let matrix = ConversionMatrix::load(&defs_dir().join("headmesh-matrix.toml")).unwrap();
    for pair in &matrix.pairs {
        if pair.status == PairStatus::Unproven {
            continue;
        }
        let Some(platform) = pair.target.platform() else {
            panic!(
                "promoted pair {} → {}: target is not an installed title",
                pair.source, pair.target
            );
        };
        let csv_path = repo_root()
            .join("docs/audits/data/assets")
            .join(pair.target.title())
            .join(platform)
            .join("capability.csv");
        let rows = read_capability_csv(&csv_path).unwrap_or_else(|| {
            panic!(
                "promoted pair {} → {} but no capability CSV at {}",
                pair.source,
                pair.target,
                csv_path.display()
            )
        });
        let heads = rows.get("heads").unwrap_or_else(|| {
            panic!(
                "promoted pair {} → {} but no `heads` row in {}",
                pair.source,
                pair.target,
                csv_path.display()
            )
        });
        let csv_game_verified = heads.get("game_verified").map(String::as_str) == Some("pass");
        let matrix_game_verified = pair.status == PairStatus::GameVerified;
        assert_eq!(
            matrix_game_verified,
            csv_game_verified,
            "pair {} → {}: matrix says {:?} but CSV heads.game_verified = {:?}",
            pair.source,
            pair.target,
            pair.status,
            heads.get("game_verified")
        );
    }
}

#[test]
fn matrix_uv_classes_are_measured_when_referenced() {
    #[derive(serde::Deserialize)]
    struct GameClass {
        key: String,
        uv_class: String,
    }
    #[derive(serde::Deserialize)]
    struct ClassesFile {
        schema: String,
        #[serde(default, rename = "game")]
        games: Vec<GameClass>,
    }
    let text = std::fs::read_to_string(defs_dir().join("head-uv-classes.toml")).unwrap();
    let classes: ClassesFile = toml::from_str(&text).unwrap();
    assert_eq!(classes.schema, "headmesh-uv-classes/1");
    let by_key: HashMap<&str, &str> = classes
        .games
        .iter()
        .map(|g| (g.key.as_str(), g.uv_class.as_str()))
        .collect();

    let matrix = ConversionMatrix::load(&defs_dir().join("headmesh-matrix.toml")).unwrap();
    for pair in &matrix.pairs {
        for (label, class, game) in [
            ("uv_class_source", &pair.uv_class_source, &pair.source),
            ("uv_class_target", &pair.uv_class_target, &pair.target),
        ] {
            if let Some(class) = class {
                assert_ne!(
                    class, "unmeasured",
                    "pair {} → {}: {label} must never be set to \"unmeasured\" — omit it instead",
                    pair.source, pair.target
                );
                assert_eq!(
                    by_key.get(game.0.as_str()).copied(),
                    Some(class.as_str()),
                    "pair {} → {}: {label}={class:?} disagrees with head-uv-classes.toml for {game}",
                    pair.source,
                    pair.target
                );
            }
        }
    }
}
