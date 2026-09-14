//! Field-level schema diff across the catalog. Two comparison sets, both derived purely from the
//! snapshots:
//!   * **backward drift** — every big-endian title (NHL 07–15 on Xbox 360) vs the NHL Legacy baseline,
//!     and every PS4 title (NHL 15–20) vs the NHL 21 baseline;
//!   * **platform parity** — each PS3 main DB vs its Xbox 360 twin (should be empty; any row is a real
//!     divergence in the "PS3 = Xbox 360 database" claim).
//!
//! Each row is a single change: a table added/removed, or a field added/removed/resized/retyped/
//! renamed. Rows are sorted, so the CSV is deterministic.

use std::collections::BTreeMap;

use crate::{schema, schema::SchemaSnapshot, MainAnchor};

struct Row {
    family: String,
    baseline: String,
    compare_platform: String,
    compare_game: String,
    table_raw: String,
    table_name: String,
    field_raw: String,
    field_name: String,
    change: String,
    detail: String,
}

fn diff_snapshots(
    family: &str,
    baseline: &SchemaSnapshot,
    cmp: &SchemaSnapshot,
    rows: &mut Vec<Row>,
) {
    let baseline_label = format!("{}/{}", baseline.platform, baseline.game);
    let b_tables: BTreeMap<&str, &schema::TableSnapshot> = baseline
        .tables
        .iter()
        .map(|t| (t.raw_id.as_str(), t))
        .collect();
    let c_tables: BTreeMap<&str, &schema::TableSnapshot> =
        cmp.tables.iter().map(|t| (t.raw_id.as_str(), t)).collect();

    let push = |table_raw: &str,
                table_name: &Option<String>,
                field_raw: &str,
                field_name: &Option<String>,
                change: &str,
                detail: String,
                rows: &mut Vec<Row>| {
        rows.push(Row {
            family: family.to_string(),
            baseline: baseline_label.clone(),
            compare_platform: cmp.platform.clone(),
            compare_game: cmp.game.clone(),
            table_raw: table_raw.to_string(),
            table_name: table_name.clone().unwrap_or_default(),
            field_raw: field_raw.to_string(),
            field_name: field_name.clone().unwrap_or_default(),
            change: change.to_string(),
            detail,
        });
    };

    // Tables only in one side.
    for (raw, t) in &b_tables {
        if !c_tables.contains_key(raw) {
            push(
                raw,
                &t.name,
                "",
                &None,
                "table-removed",
                format!("{} fields", t.field_count),
                rows,
            );
        }
    }
    for (raw, t) in &c_tables {
        if !b_tables.contains_key(raw) {
            push(
                raw,
                &t.name,
                "",
                &None,
                "table-added",
                format!("{} fields", t.field_count),
                rows,
            );
        }
    }

    // Fields within shared tables.
    for (raw, bt) in &b_tables {
        let Some(ct) = c_tables.get(raw) else {
            continue;
        };
        let b_fields: BTreeMap<&str, &schema::FieldSnapshot> =
            bt.fields.iter().map(|f| (f.raw_id.as_str(), f)).collect();
        let c_fields: BTreeMap<&str, &schema::FieldSnapshot> =
            ct.fields.iter().map(|f| (f.raw_id.as_str(), f)).collect();
        for (fraw, bf) in &b_fields {
            match c_fields.get(fraw) {
                None => push(
                    raw,
                    &bt.name,
                    fraw,
                    &bf.name,
                    "field-removed",
                    String::new(),
                    rows,
                ),
                Some(cf) => {
                    if bf.bit_width != cf.bit_width {
                        push(
                            raw,
                            &bt.name,
                            fraw,
                            &bf.name,
                            "field-resized",
                            format!("{}->{} bits", bf.bit_width, cf.bit_width),
                            rows,
                        );
                    }
                    if bf.meta_type.is_some()
                        && cf.meta_type.is_some()
                        && bf.meta_type != cf.meta_type
                    {
                        push(
                            raw,
                            &bt.name,
                            fraw,
                            &bf.name,
                            "field-retyped",
                            format!("{}->{}", opt(&bf.meta_type), opt(&cf.meta_type)),
                            rows,
                        );
                    }
                    if bf.name.is_some() && cf.name.is_some() && bf.name != cf.name {
                        push(
                            raw,
                            &bt.name,
                            fraw,
                            &bf.name,
                            "field-renamed",
                            format!("{}->{}", opt(&bf.name), opt(&cf.name)),
                            rows,
                        );
                    }
                }
            }
        }
        for (fraw, cf) in &c_fields {
            if !b_fields.contains_key(fraw) {
                push(
                    raw,
                    &ct.name,
                    fraw,
                    &cf.name,
                    "field-added",
                    String::new(),
                    rows,
                );
            }
        }
    }
}

fn opt(o: &Option<String>) -> String {
    o.clone().unwrap_or_default()
}

fn csv_cell(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Build the full schema-diff CSV from the discovered main-DB anchors.
pub fn build_csv(repo_root: &std::path::Path, anchors: &[MainAnchor]) -> Result<String, String> {
    // Snapshot every anchor once, keyed by (platform, game).
    let mut snaps: BTreeMap<(String, String), SchemaSnapshot> = BTreeMap::new();
    for a in anchors {
        snaps.insert(
            (a.platform.clone(), a.game.clone()),
            schema::snapshot(repo_root, a)?,
        );
    }
    let get = |platform: &str, game: &str| snaps.get(&(platform.to_string(), game.to_string()));

    let mut rows: Vec<Row> = Vec::new();

    // Backward drift vs the big-endian Legacy baseline (Xbox 360). NHL 07 is gen6 (a large gap);
    // NHL 08–15 are gen7. PS3 twins are covered by the platform-parity set below.
    if let Some(legacy) = get("xbox360", "nhllegacy") {
        for game in [
            "nhl07", "nhl08", "nhl09", "nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15",
        ] {
            if let Some(cmp) = get("xbox360", game) {
                let family = if game == "nhl07" {
                    "gen6-vs-legacy"
                } else {
                    "gen7-vs-legacy"
                };
                diff_snapshots(family, legacy, cmp, &mut rows);
            }
        }
    }

    // PS4 backward drift vs the NHL 21 baseline.
    if let Some(base) = get("ps4", "nhl21") {
        for game in ["nhl15", "nhl16", "nhl17", "nhl18", "nhl19", "nhl20"] {
            if let Some(cmp) = get("ps4", game) {
                diff_snapshots("gen8-vs-nhl21", base, cmp, &mut rows);
            }
        }
    }

    // Platform parity: PS3 vs Xbox 360 per big-endian game (expected empty).
    for game in [
        "nhl07",
        "nhl08",
        "nhl09",
        "nhl10",
        "nhl11",
        "nhl12",
        "nhl13",
        "nhl14",
        "nhl15",
        "nhllegacy",
    ] {
        if let (Some(xbox), Some(ps3)) = (get("xbox360", game), get("ps3", game)) {
            diff_snapshots("platform-parity", xbox, ps3, &mut rows);
        }
    }

    rows.sort_by(|a, b| {
        (
            &a.family,
            &a.compare_game,
            &a.compare_platform,
            &a.table_raw,
            &a.field_raw,
            &a.change,
        )
            .cmp(&(
                &b.family,
                &b.compare_game,
                &b.compare_platform,
                &b.table_raw,
                &b.field_raw,
                &b.change,
            ))
    });

    let mut s = String::from(
        "family,baseline,compare_platform,compare_game,table_raw,table_name,field_raw,field_name,change,detail\n",
    );
    for r in &rows {
        s.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            csv_cell(&r.family),
            csv_cell(&r.baseline),
            csv_cell(&r.compare_platform),
            csv_cell(&r.compare_game),
            csv_cell(&r.table_raw),
            csv_cell(&r.table_name),
            csv_cell(&r.field_raw),
            csv_cell(&r.field_name),
            csv_cell(&r.change),
            csv_cell(&r.detail),
        ));
    }
    Ok(s)
}
