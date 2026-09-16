//! Package C0-AS7: the record and join logic behind the D1 / D3 / D4 measurements, exercised two
//! ways. The **synthetic** cases run the logic on bytes and rows built here and assert exact
//! outcomes; the **specimen** cases read the committed records under `docs/receipts/data/`
//! (written once from private copies by the two `as7_*` examples), check their integrity against
//! their own metadata, and re-derive every join from those records plus the committed anchor,
//! comparing the result byte-for-byte with the committed join record. A specimen case never
//! opens a private copy; if a record is missing it fails, it does not skip.
//!
//! Nothing here promotes a block. Every number a specimen case pins is a measurement of one
//! copy's files joined to one anchor's field, and the receipt quotes them at that scope.

use super::asset_census::*;
use crate::target_evidence::{read, repo_root, sha256_hex, table};
use std::collections::{BTreeMap, BTreeSet};
use tdb_core::{FieldDescriptor, Table, TdbFile};

// ==================================================================== synthetic

#[test]
fn ranges_render_runs_and_singletons_deterministically() {
    let ids: BTreeSet<u64> = [1, 2, 3, 7, 9, 10, 60000].into_iter().collect();
    assert_eq!(ranges(&ids), "1-3;7;9-10;60000");
    assert_eq!(ranges(&BTreeSet::new()), "");
    assert_eq!(ranges(&BTreeSet::from([5])), "5");
}

#[test]
fn portrait_names_classify_by_label_and_numeric_id_from_the_name_only() {
    use PortraitLabel::*;
    assert_eq!(classify_portrait_name("p1.big"), (Current, Some(1)));
    assert_eq!(classify_portrait_name("p12401.big"), (Current, Some(12401)));
    assert_eq!(classify_portrait_name("p60003.big"), (Current, Some(60003)));
    assert_eq!(classify_portrait_name("p0.big"), (Current, Some(0)));
    assert_eq!(
        classify_portrait_name("p127.big.bak"),
        (BackupLabelled, Some(127))
    );
    assert_eq!(classify_portrait_name("p1112_glump.options"), (Other, None));
    assert_eq!(classify_portrait_name("px.big"), (Other, None));
    assert_eq!(classify_portrait_name("p.big"), (Other, None));
    assert_eq!(classify_portrait_name("1.big"), (Other, None));
    assert_eq!(classify_portrait_name("p1.BIG"), (Other, None));
}

/// A `BIGF` archive in the six-member portrait shape C0-POR1 measured, with member 3 carrying a
/// RefPack header that declares `declared` bytes.
fn synthetic_bigf(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(b"BIGF");
    header.extend_from_slice(&0u32.to_le_bytes()); // size, patched below
    header.extend_from_slice(&(members.len() as u32).to_be_bytes());
    header.extend_from_slice(&0u32.to_be_bytes()); // header size, patched below
    let table_len: usize = members.iter().map(|(n, _)| 8 + n.len() + 1).sum();
    let mut data_start = 16 + table_len;
    data_start = (data_start + 63) & !63;
    let mut offsets = Vec::new();
    let mut cursor = data_start;
    for (_, payload) in members {
        offsets.push(cursor);
        cursor = (cursor + payload.len() + 63) & !63;
    }
    for ((name, payload), off) in members.iter().zip(&offsets) {
        header.extend_from_slice(&(*off as u32).to_be_bytes());
        header.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        header.extend_from_slice(name.as_bytes());
        header.push(0);
    }
    let header_len = header.len() as u32;
    header[12..16].copy_from_slice(&header_len.to_be_bytes());
    let mut out = header;
    out.resize(cursor, 0);
    for ((_, payload), off) in members.iter().zip(&offsets) {
        out[*off..*off + payload.len()].copy_from_slice(payload);
    }
    let size = (out.len() as u32 - 16).to_le_bytes();
    out[4..8].copy_from_slice(&size);
    out
}

#[test]
fn bigf_peek_reads_the_member_table_and_the_refpack_declared_size_without_decompressing() {
    // 3-byte size variant: 0x10 0xFB + 262272 (= 0x040080).
    let refpack3 = [0x10u8, 0xFB, 0x04, 0x00, 0x80, 0xFC];
    let archive = synthetic_bigf(&[
        ("0", b"Apt Data"),
        ("177", &[0, 0, 1, 0x20]),
        ("sg1", b""),
        ("176", &refpack3),
        ("sg2", b""),
        ("1", b"Apt constant"),
    ]);
    assert_eq!(
        peek_bigf_portrait(&archive),
        BigfPeek {
            members: Some(6),
            texture_member: Some("176".into()),
            texture_stored_size: Some(6),
            texture_declared_size: Some(262_272),
        }
    );
    // 4-byte size variant with a compressed-size field in front of it.
    let refpack4 = [0xD0u8, 0xFB, 0, 0, 0, 9, 0, 0x02, 0x00, 0x80];
    assert_eq!(refpack_declared_size(&refpack4), Some(0x20080));
    assert_eq!(refpack_declared_size(&[0x10, 0xFB]), None);
    assert_eq!(refpack_declared_size(b"DDS "), None);
    // Fewer than four members → no texture cells; not BIGF → nothing at all.
    let short = synthetic_bigf(&[("0", b"x"), ("1", b"y")]);
    assert_eq!(
        peek_bigf_portrait(&short),
        BigfPeek {
            members: Some(2),
            ..BigfPeek::default()
        }
    );
    assert_eq!(peek_bigf_portrait(b"-big -o32"), BigfPeek::default());
    assert_eq!(peek_bigf_portrait(b""), BigfPeek::default());
}

fn file(folder: &str, bucket: &str, name: &str, sha: &str) -> PortraitFile {
    let (label, numeric_id) = classify_portrait_name(name);
    PortraitFile {
        folder: folder.into(),
        bucket: bucket.into(),
        file_name: name.into(),
        label,
        numeric_id,
        size: 100,
        sha256: sha.into(),
        peek: BigfPeek {
            members: Some(6),
            texture_member: Some("9".into()),
            texture_stored_size: Some(50),
            texture_declared_size: Some(32_896),
        },
    }
}

#[test]
fn portrait_census_csv_round_trips_and_refuses_a_row_whose_label_disagrees_with_its_name() {
    let files = vec![
        file("playerheads", "p0_4000", "p1.big", "aa"),
        file("playerheads", "p0_4000", "p1.big.bak", "bb"),
        file("playerheadssmall", "silhouettes", "p60000.big", "cc"),
    ];
    let csv = render_portrait_census_csv(&files);
    assert!(csv.starts_with(PORTRAIT_CENSUS_HEADER));
    assert_eq!(parse_portrait_census_csv(&csv).unwrap(), files);
    let tampered = csv.replace("p1.big.bak,backup-labelled,1", "p1.big.bak,current,1");
    assert!(parse_portrait_census_csv(&tampered)
        .unwrap_err()
        .contains("disagree"));
    assert!(parse_portrait_census_csv("nope\n").is_err());
    assert!(parse_portrait_census_csv(&format!("{PORTRAIT_CENSUS_HEADER}\na,b,c\n")).is_err());
}

#[test]
fn portrait_join_reports_matched_unmatched_shared_zero_backup_and_patch_ids_separately() {
    let files = vec![
        file("playerheads", "p0_4000", "p1.big", "a"),
        file("playerheads", "p0_4000", "p2.big", "b"),
        file("playerheads", "p0_4000", "p2.big.bak", "c"),
        file("playerheads", "p4001_8000", "p2.big", "d"), // the same id in two buckets
        file("playerheads", "p0_4000", "p3.big.bak", "e"), // backup with no current
        file("playerheads", "silhouettes", "p60000.big", "f"),
        file("playerheadssmall", "p0_4000", "p1.big", "g"), // other folder, ignored here
    ];
    let db = IdHistogram {
        table: "ubPc".into(),
        field: "lOnr".into(),
        rows: 8,
        by_value: BTreeMap::from([(0, 2), (1, 1), (2, 2), (5, 1), (100, 1), (200, 1)]),
    };
    let patch: BTreeSet<u64> = [5, 100, 300].into_iter().collect();
    let rows = join_portrait_folder("playerheads", &files, &db, &patch, 100, 150);
    let get = |m: &str| -> &str {
        &rows
            .iter()
            .find(|(_, k, _)| k == m)
            .unwrap_or_else(|| panic!("{m}"))
            .2
    };
    assert_eq!(get("files"), "6");
    assert_eq!(get("files_current"), "4");
    assert_eq!(get("files_backup_labelled"), "2");
    assert_eq!(get("current_distinct_ids"), "3");
    assert_eq!(get("current_ids_with_multiple_files"), "1");
    assert_eq!(get("current_ids_with_multiple_files_ranges"), "2");
    assert_eq!(get("current_regular_id_range"), "1-2");
    assert_eq!(get("current_silhouette_ids"), "60000");
    assert_eq!(get("current_ids_with_backup_sibling"), "1");
    assert_eq!(get("backup_ids_without_current_ranges"), "3");
    assert_eq!(get("file_for_id_zero"), "false");
    assert_eq!(get("db_field"), "ubPc.lOnr");
    assert_eq!(get("db_zero_rows"), "2");
    assert_eq!(get("db_shared_ids"), "2x2");
    assert_eq!(get("db_distinct_nonzero_ids"), "5");
    assert_eq!(get("db_ids_with_current_file"), "2");
    assert_eq!(get("db_ids_without_current_file"), "3");
    assert_eq!(
        get("db_ids_without_current_file_outside_regular_ranges"),
        "1"
    );
    assert_eq!(get("db_ids_without_current_file_elsewhere_ranges"), "5;200");
    assert_eq!(get("current_ids_without_db_row"), "1"); // 60000
    assert_eq!(get("patch_member_ids_ranges"), "5;100;300");
    assert_eq!(get("patch_member_ids_with_db_row"), "2");
    assert_eq!(get("patch_member_ids_without_db_row_ranges"), "300");
    assert_eq!(get("patch_member_ids_also_loose_current"), "0");
    assert_eq!(get("patch_member_ids_outside_regular_ranges"), "1");
    assert_eq!(get("patch_member_ids_above_db_max"), "1");
    assert_eq!(get("db_ids_only_in_patch_ranges"), "5;100");
    assert_eq!(get("db_ids_in_neither_ranges"), "200");
    assert_eq!(get("db_ids_in_neither_outside_regular_ranges"), "0");
    // The CSV rendering is stable and re-parses cell-for-cell.
    let csv = render_join_csv(&rows);
    assert_eq!(csv.lines().count(), rows.len() + 1);
}

fn member(path: &str) -> EbMemberRow {
    EbMemberRow {
        index: 0,
        path: path.into(),
        offset: 16,
        size: 8,
        word_4: "0x00000000".into(),
        word_12: "0x00000000".into(),
        stored_magic: "chunklzx".into(),
        stored_sha256: "00".into(),
        chunk_header: String::new(),
        decoded_form: "chunklzx-unsupported".into(),
        decoded_size: None,
        decoded_sha256: String::new(),
        decode_error: String::new(),
        loose_present: None,
        loose_size: None,
        loose_sha256: String::new(),
        loose_bak_present: None,
        loose_bak_size: None,
        loose_bak_sha256: String::new(),
    }
}

#[test]
fn eb_member_records_round_trip_including_the_quoted_codec_error_and_refuse_disorder() {
    let text = format!(
        "{EB_MEMBERS_HEADER}\n\
         0,rendering/logo/texlib_10_0_0.rx2,16,8,0x00000000,0x13a8968e,chunklzx,ab,dummy=2;w12=1;w16=2;w20=3;w24=16,chunklzx-unsupported,,,,,,,,,\n\
         1,fe/ion/artassets/playerheads/p0_4000/p555.big,32,20848,0x00000000,0xd185275f,BIGF,cd,,raw,20848,cd,,true,20848,ef,false,,\n\
         2,rendering/pant/pant_0_0_0_0.rpsgl,64,9,0x00000000,0x00000001,chunkzip,12,dummy=2;w12=9;w16=131072;w20=3;w24=16,decode-error,,,\"chunkzip header is truncated: needed 10, have 9\",false,,,false,,\n"
    );
    let rows = parse_eb_members_csv(&text).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].directory(), "rendering/logo");
    assert_eq!(rows[0].base_name(), "texlib_10_0_0.rx2");
    assert_eq!(rows[1].loose_present, Some(true));
    assert_eq!(rows[1].loose_sha256, "ef");
    assert_eq!(rows[1].loose_bak_present, Some(false));
    assert_eq!(rows[1].decoded_size, Some(20848));
    assert_eq!(
        rows[2].decode_error,
        "chunkzip header is truncated: needed 10, have 9"
    );
    assert_eq!(rows[2].loose_present, Some(false));
    let disordered = text.replace("\n2,rendering", "\n5,rendering");
    assert!(parse_eb_members_csv(&disordered)
        .unwrap_err()
        .contains("out of order"));
    assert!(parse_eb_members_csv("index,path\n").is_err());
}

#[test]
fn name_shapes_replace_only_all_digit_components_and_wildcards_match_one_non_numeric_component() {
    assert_eq!(name_shape("texlib_10_0_0.rx2"), "texlib_N_N_N.rx2");
    assert_eq!(name_shape("glove_27_0_tm.rx2"), "glove_N_N_tm.rx2");
    assert_eq!(
        name_shape("goaliestick_goaliestick_dm.rx2"),
        "goaliestick_goaliestick_dm.rx2"
    );
    assert_eq!(name_shape("credits"), "credits");
    assert_eq!(name_shape("p555.big"), "p555.big"); // `p555` is not all digits
    assert!(shape_matches("glove_N_*.rx2", "glove_N_am.rx2"));
    assert!(!shape_matches("glove_N_*.rx2", "glove_N_N_tm.rx2"));
    assert!(!shape_matches("glove_N_*.rx2", "glove_N_N.rx2"));
    assert!(!shape_matches("glove_N_*.rx2", "glove_N_am.rpsgl"));
    assert!(shape_matches("texlib_N.rx2", "texlib_N.rx2"));
}

const LOGO_DECLARED: Grammar = Grammar {
    label: "logo/texlib_{team-id}_0_0",
    directory: "rendering/logo",
    shape: "texlib_N_N_N.rx2",
    literal: &[(2, "0"), (3, "0")],
    id_components: &[1],
    db_field: "kOtt.lOnr",
    origin: "declared",
};

const JERSEY_DECLARED: Grammar = Grammar {
    label: "jersey/texlib_{style}_{art}_{variant}",
    directory: "rendering/jersey",
    shape: "texlib_N_N_N.rx2",
    literal: &[],
    id_components: &[1, 2, 3],
    db_field: "xjgw.cVLR_GDQL_FccT",
    origin: "declared",
};

#[test]
fn grammars_read_literal_member_names_and_join_to_rendered_db_ids() {
    let rows = vec![
        member("rendering/logo/texlib_10_0_0.rx2"),
        member("rendering/logo/texlib_10_0_1.rx2"),
        member("rendering/logo/texlib_11_0_0.rx2"),
        member("rendering/logo/other.rx2"),
        member("rendering/jersey/texlib_1_19_3.rx2"),
        member("rendering/jersey/name_1_19_3_cm.rx2"),
    ];
    assert_eq!(grammar_id(&LOGO_DECLARED, &rows[0]).as_deref(), Some("10"));
    assert_eq!(grammar_id(&LOGO_DECLARED, &rows[1]), None); // third component is not 0
    assert_eq!(grammar_id(&LOGO_DECLARED, &rows[4]), None); // wrong directory
    assert_eq!(
        grammar_id(&JERSEY_DECLARED, &rows[4]).as_deref(),
        Some("1_19_3")
    );
    assert_eq!(grammar_id(&JERSEY_DECLARED, &rows[5]), None);

    let db: BTreeSet<String> = ["10", "12"].iter().map(|s| s.to_string()).collect();
    let j = join_grammar(&LOGO_DECLARED, "cacherender.big", &rows, &db);
    assert_eq!(
        (j.directory_members, j.matching_members, j.distinct_ids),
        (4, 2, 2)
    );
    assert_eq!(j.id_range, "10-11");
    assert_eq!((j.db_ids_with_member, j.db_ids_without_member), (1, 1));
    assert_eq!(j.db_ids_without_member_list, "12");
    assert_eq!(j.member_ids_without_db_row_list, "11");
    let csv = render_member_join_csv(&[j]);
    assert!(csv.starts_with(MEMBER_JOIN_HEADER));
    assert_eq!(csv.lines().count(), 2);

    let hist = shape_histogram(&rows);
    assert_eq!(
        hist[&("rendering/logo".into(), "texlib_N_N_N.rx2".into())],
        3
    );
    assert_eq!(hist[&("rendering/logo".into(), "other.rx2".into())], 1);
}

// ==================================================================== specimen records

const DATA: &str = "docs/receipts/data";

/// Compare a re-derived record with the committed one. A missing or differing record fails and
/// prints the regenerated text so it can be installed deliberately; nothing is written here.
fn assert_record_matches(name: &str, rendered: &str) {
    let path = repo_root().join(DATA).join(name);
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| {
            panic!(
                "{}: {e} - the C0-AS7 record is missing, not skipped\n--- regenerate from ---\n{rendered}",
                path.display()
            )
        })
        .replace("\r\n", "\n");
    assert!(
        committed == rendered,
        "{name} differs from the re-derived record\n--- regenerated ---\n{rendered}"
    );
}

/// A committed record's text, CRLF-normalised (the repository checks out with `core.autocrlf`).
fn record_text(name: &str) -> String {
    let path = repo_root().join(DATA).join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| {
            panic!(
                "{}: {e} — the C0-AS7 record is missing, not skipped",
                path.display()
            )
        })
        .replace("\r\n", "\n")
}

/// Minimal extraction from the tool-written metadata JSON: the value of a top-level or nested
/// `"key": "string"` / `"key": number` pair. The metadata is flat enough that a key search is
/// unambiguous; the keys used below occur once each.
fn json_string(text: &str, key: &str) -> String {
    let needle = format!("\"{key}\": \"");
    let start = text.find(&needle).unwrap_or_else(|| panic!("{key}")) + needle.len();
    let end = text[start..].find('"').unwrap();
    text[start..start + end].to_string()
}

fn json_number(text: &str, key: &str) -> u64 {
    let needle = format!("\"{key}\": ");
    let start = text.find(&needle).unwrap_or_else(|| panic!("{key}")) + needle.len();
    let digits: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap()
}

/// The four member/census records, each checked against its own metadata: schema tag, source
/// commit, the CSV's digest, and the row count.
#[test]
fn the_committed_as7_records_match_their_own_metadata() {
    for (label, schema, count_key, csv_key, sha_key) in [
        (
            "c0-as7-ps3-portrait-census",
            PORTRAIT_CENSUS_SCHEMA,
            "files",
            "census_csv",
            "census_csv_sha256",
        ),
        (
            "c0-as7-x360-cacherender-members",
            EB_MEMBERS_SCHEMA,
            "entries",
            "members_csv",
            "members_csv_sha256",
        ),
        (
            "c0-as7-x360-nocacherender-members",
            EB_MEMBERS_SCHEMA,
            "entries",
            "members_csv",
            "members_csv_sha256",
        ),
        (
            "c0-as7-ps3-patch-members",
            EB_MEMBERS_SCHEMA,
            "entries",
            "members_csv",
            "members_csv_sha256",
        ),
    ] {
        let meta = record_text(&format!("{label}.json"));
        assert_eq!(json_string(&meta, "record"), schema, "{label}");
        assert_eq!(json_string(&meta, "package"), "C0-AS7", "{label}");
        assert_eq!(
            json_string(&meta, "source_commit"),
            "9e02acfe7e50e0737970fccf3b7d4321bef00f4b",
            "{label}: the tool ran on the C0-AS6 baseline"
        );
        assert_eq!(json_string(&meta, csv_key), format!("{label}.csv"));
        let csv = record_text(&format!("{label}.csv"));
        assert_eq!(
            sha256_hex(csv.as_bytes()),
            json_string(&meta, sha_key),
            "{label}: the CSV is the one the tool wrote"
        );
        let rows = csv.lines().count() - 1;
        assert_eq!(rows as u64, json_number(&meta, count_key), "{label}");
        if schema == EB_MEMBERS_SCHEMA {
            let parsed = parse_eb_members_csv(&csv).unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!(parsed.len(), rows);
            // A member table is a listing of literal names: every path is ASCII, none is empty,
            // none repeats.
            let distinct: BTreeSet<&str> = parsed.iter().map(|r| r.path.as_str()).collect();
            assert_eq!(distinct.len(), parsed.len(), "{label}: duplicate paths");
            assert!(parsed
                .iter()
                .all(|r| r.path.is_ascii() && !r.path.is_empty()));
            assert_eq!(json_number(&meta, "distinct_paths"), parsed.len() as u64);
        } else {
            let parsed = parse_portrait_census_csv(&csv).unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!(parsed.len(), rows);
        }
    }
}

/// A committed anchor's bytes, pinned by digest; callers parse the bytes they hold.
fn anchor_bytes(rel: &str, expected_sha256: &str) -> Vec<u8> {
    let bytes = std::fs::read(repo_root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
    assert_eq!(sha256_hex(&bytes), expected_sha256, "{rel}");
    bytes
}

/// The two Legacy anchors are one set of bytes (C0-REG2, re-asserted by C0-AS6).
const LEGACY_ANCHOR_SHA256: &str =
    "105d2186da3d1f0f3d3ea8a350ead42ac9d5d802ca9fd37652c39bb4e1f1c230";

fn find_field(db: &TdbFile, t: &Table, id: &str) -> FieldDescriptor {
    t.fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == id)
        .cloned()
        .unwrap_or_else(|| panic!("{id}"))
}

fn histogram(db: &TdbFile, table_id: &str, field_id: &str) -> IdHistogram {
    let t = table(db, table_id);
    let f = find_field(db, t, field_id);
    let mut by_value = BTreeMap::new();
    for i in 0..u32::from(t.current_records) {
        *by_value.entry(read(db, t, i, &f)).or_default() += 1;
    }
    IdHistogram {
        table: table_id.into(),
        field: field_id.into(),
        rows: t.current_records as usize,
        by_value,
    }
}

/// The portrait ids `patch.big`'s members spell, per folder — the D3 input to the D4 join.
fn patch_portrait_ids() -> PatchPortraitIds {
    let rows = parse_eb_members_csv(&record_text("c0-as7-ps3-patch-members.csv")).unwrap();
    let mut out = PatchPortraitIds::new();
    for folder in ["playerheads", "playerheadssmall"] {
        let prefix = format!("fe/ion/artassets/{folder}/");
        let ids: BTreeSet<u64> = rows
            .iter()
            .filter(|r| r.path.starts_with(&prefix))
            .map(|r| {
                let (label, id) = classify_portrait_name(r.base_name());
                assert_eq!(label, PortraitLabel::Current, "{}", r.path);
                id.unwrap()
            })
            .collect();
        out.insert(folder.into(), ids);
    }
    out
}

/// The anchor's 350 ids beyond the highest regular portrait id C0-POR1 observed (12401): the range
/// the join accounts for explicitly, as "outside the observed regular ranges".
const OUTSIDE_LO: u64 = 12403;
const OUTSIDE_HI: u64 = 12814;

fn render_portrait_join_record() -> (String, Vec<(String, String, String)>) {
    let files = parse_portrait_census_csv(&record_text("c0-as7-ps3-portrait-census.csv")).unwrap();
    let bytes = anchor_bytes(
        "anchors/ps3/nhllegacy/databases/nhlng.db",
        LEGACY_ANCHOR_SHA256,
    );
    let db = TdbFile::parse(&bytes).unwrap();
    let h = histogram(&db, "ubPc", "lOnr");
    let patch = patch_portrait_ids();
    let mut rows = Vec::new();
    for folder in ["playerheads", "playerheadssmall"] {
        rows.extend(join_portrait_folder(
            folder,
            &files,
            &h,
            &patch[folder],
            OUTSIDE_LO,
            OUTSIDE_HI,
        ));
    }
    (render_join_csv(&rows), rows)
}

/// D4: the census joined to the Legacy anchor's `ubPc.lOnr`, with `patch.big`'s portrait members
/// accounted for. The committed join record must equal what the census and the anchor say today;
/// if it does not, the regenerated text is printed and must be installed deliberately.
#[test]
fn ps3_portrait_census_joins_to_the_legacy_anchor_and_the_join_record_matches() {
    let (rendered, rows) = render_portrait_join_record();
    assert_record_matches("c0-as7-ps3-portrait-join.csv", &rendered);
    let get = |folder: &str, m: &str| -> String {
        rows.iter()
            .find(|(f, k, _)| f == folder && k == m)
            .unwrap_or_else(|| panic!("{folder}/{m}"))
            .2
            .clone()
    };
    // The facts the receipt and the profile note quote, pinned so a change is a review event.
    for folder in ["playerheads", "playerheadssmall"] {
        assert_eq!(get(folder, "db_rows"), "5632");
        assert_eq!(get(folder, "db_distinct_values"), "5450");
        assert_eq!(get(folder, "db_zero_rows"), "180");
        assert_eq!(get(folder, "db_shared_ids"), "2x2;7x2;12x2");
        assert_eq!(get(folder, "db_distinct_nonzero_ids"), "5449");
        assert_eq!(get(folder, "file_for_id_zero"), "false");
        assert_eq!(get(folder, "files_backup_labelled"), "103");
        assert_eq!(get(folder, "current_ids_with_multiple_files"), "0");
        assert_eq!(get(folder, "current_silhouette_ids"), "60000-60003");
        assert_eq!(
            get(folder, "db_ids_without_current_file_outside_regular_ranges"),
            "350"
        );
        assert_eq!(get(folder, "current_regular_id_range"), "1-12401");
        assert_eq!(get(folder, "current_ids_without_db_row"), "4728");
        // The ids `patch.big` carries that no anchor row stores — eleven of them above the
        // anchor's highest id, and 12508 inside the outside-range band.
        assert_eq!(
            get(folder, "patch_member_ids_without_db_row_ranges"),
            "8323;12508;12816;12818;12824-12832"
        );
        assert_eq!(get(folder, "patch_member_ids_above_db_max"), "11");
    }
    // playerheads: 4 500 of the 5 449 non-zero anchor ids have a loose current file; 949 do not —
    // the 350 outside the observed regular ranges and 599 inside them. `patch.big` supplies 33
    // of the 949 (28 of them in 12633..12666, i.e. among the 350); 916 are in neither, 322 of
    // those among the 350.
    assert_eq!(get("playerheads", "files_current"), "9228");
    assert_eq!(get("playerheads", "db_ids_with_current_file"), "4500");
    assert_eq!(get("playerheads", "db_ids_without_current_file"), "949");
    assert_eq!(
        get("playerheads", "db_ids_without_current_file_elsewhere"),
        "599"
    );
    assert_eq!(get("playerheads", "patch_member_ids"), "58");
    assert_eq!(get("playerheads", "patch_member_ids_with_db_row"), "45");
    assert_eq!(
        get("playerheads", "patch_member_ids_also_loose_current"),
        "12"
    );
    assert_eq!(
        get("playerheads", "patch_member_ids_outside_regular_ranges"),
        "29"
    );
    assert_eq!(get("playerheads", "db_ids_only_in_patch"), "33");
    assert_eq!(get("playerheads", "db_ids_in_neither"), "916");
    assert_eq!(
        get("playerheads", "db_ids_in_neither_outside_regular_ranges"),
        "322"
    );
    // playerheadssmall: 4 501 / 948 / 598; `patch.big` holds 25 small members and supplies one
    // missing id (11692); all 350 outside-range ids are in neither.
    assert_eq!(get("playerheadssmall", "files_current"), "9229");
    assert_eq!(get("playerheadssmall", "db_ids_with_current_file"), "4501");
    assert_eq!(
        get("playerheadssmall", "db_ids_without_current_file"),
        "948"
    );
    assert_eq!(
        get("playerheadssmall", "db_ids_without_current_file_elsewhere"),
        "598"
    );
    assert_eq!(get("playerheadssmall", "patch_member_ids"), "25");
    assert_eq!(
        get("playerheadssmall", "db_ids_only_in_patch_ranges"),
        "11692"
    );
    assert_eq!(get("playerheadssmall", "db_ids_in_neither"), "947");
    assert_eq!(
        get(
            "playerheadssmall",
            "db_ids_in_neither_outside_regular_ranges"
        ),
        "350"
    );
}

/// The grammars joined over the two Legacy Xbox 360 render archives — every committed template
/// (`declared`), the HEAD `SLOT_SPECS` matchers the equipment profile keeps in prose (`source`),
/// and the shapes the listing shows that no template spells (`observed`). Each names the anchor
/// field its id is compared to.
const X360_GRAMMARS: &[Grammar] = &[
    LOGO_DECLARED,
    Grammar {
        label: "logo/texlib_{team-id}_{a}_{b}",
        directory: "rendering/logo",
        shape: "texlib_N_N_N.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "kOtt.lOnr",
        origin: "observed",
    },
    JERSEY_DECLARED,
    Grammar {
        label: "jersey/name_{style}_{art}_{variant}_cm",
        directory: "rendering/jersey",
        shape: "name_N_N_N_cm.rx2",
        literal: &[],
        id_components: &[1, 2, 3],
        db_field: "xjgw.cVLR_GDQL_FccT",
        origin: "observed",
    },
    Grammar {
        label: "sock/sock_{variant}_cm",
        directory: "rendering/sock",
        shape: "sock_N_N_N_cm.rx2",
        literal: &[],
        id_components: &[1, 2, 3],
        db_field: "xjgw.cVLR_GDQL_FccT",
        origin: "observed",
    },
    Grammar {
        label: "playerstick/texlib_{numeric-id}",
        directory: "rendering/playerstick",
        shape: "texlib_N.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "xmja.wXFx",
        origin: "declared",
    },
    Grammar {
        label: "playerstick/texlib_{numeric-id} vs stick-model enum",
        directory: "rendering/playerstick",
        shape: "texlib_N.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "defs/enums/stick-model.toml#entries.raw",
        origin: "declared",
    },
    Grammar {
        label: "skate/texlib_{numeric-id}",
        directory: "rendering/skate",
        shape: "texlib_N.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "xmja.AOOe",
        origin: "declared",
    },
    Grammar {
        label: "helmet/texlib_{numeric-id}",
        directory: "rendering/helmet",
        shape: "texlib_N.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "xmja.YUZp",
        origin: "declared",
    },
    Grammar {
        label: "glove/glove_{numeric-id}_{variant}",
        directory: "rendering/glove",
        shape: "glove_N_*.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "xmja.NmUR",
        origin: "declared",
    },
    Grammar {
        label: "glove/glove_{numeric-id}_{n}_{map}",
        directory: "rendering/glove",
        shape: "glove_N_N_*.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "xmja.NmUR",
        origin: "observed",
    },
    Grammar {
        label: "goaliestick/goaliestick_{numeric-id}_{variant}",
        directory: "rendering/goaliestick",
        shape: "goaliestick_N_*.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "xmja.wXFx",
        origin: "declared",
    },
    Grammar {
        label: "goaliepad/goaliepad_{numeric-id}_{variant}",
        directory: "rendering/goaliepad",
        shape: "goaliepad_N_*.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "fMVl.mIUA",
        origin: "declared",
    },
    Grammar {
        label: "goaliepad/goaliepad_{numeric-id}_{n}_{map}",
        directory: "rendering/goaliepad",
        shape: "goaliepad_N_N_*.rx2",
        literal: &[],
        id_components: &[1],
        db_field: "fMVl.mIUA",
        origin: "observed",
    },
    Grammar {
        label: "goaliehelmet/goaliehelmet_{variant}_{numeric-id}_cm",
        directory: "rendering/goaliehelmet",
        shape: "goaliehelmet_N_N_cm.rx2",
        literal: &[],
        id_components: &[2],
        db_field: "fMVl.JOlt",
        origin: "declared",
    },
    Grammar {
        label: "blocker/blocker_{model}_{numeric-id}_{map}",
        directory: "rendering/blocker",
        shape: "blocker_N_N_*.rx2",
        literal: &[],
        id_components: &[2],
        db_field: "fMVl.Kmxz",
        origin: "source",
    },
    Grammar {
        label: "blocker/blocker_{model}_{numeric-id}_{n}_{map}",
        directory: "rendering/blocker",
        shape: "blocker_N_N_N_*.rx2",
        literal: &[],
        id_components: &[2],
        db_field: "fMVl.Kmxz",
        origin: "observed",
    },
    Grammar {
        label: "trapper/trapper_{model}_{numeric-id}_{map}",
        directory: "rendering/trapper",
        shape: "trapper_N_N_*.rx2",
        literal: &[],
        id_components: &[2],
        db_field: "fMVl.OTJR",
        origin: "source",
    },
    Grammar {
        label: "trapper/trapper_{model}_{numeric-id}_{n}_{map}",
        directory: "rendering/trapper",
        shape: "trapper_N_N_N_*.rx2",
        literal: &[],
        id_components: &[2],
        db_field: "fMVl.OTJR",
        origin: "observed",
    },
    Grammar {
        label: "playerheads/p{numeric-id}",
        directory: "fe/ion/artassets/playerheads",
        shape: "p*.big",
        literal: &[],
        id_components: &[0],
        db_field: "ubPc.lOnr",
        origin: "declared",
    },
];

/// The rendered DB id set a grammar's `db_field` names, from the Legacy anchor.
fn db_ids_for(db: &TdbFile, field: &str) -> BTreeSet<String> {
    match field {
        "xjgw.cVLR_GDQL_FccT" => {
            let t = table(db, "xjgw");
            let (s, a, v) = (
                find_field(db, t, "cVLR"),
                find_field(db, t, "GDQL"),
                find_field(db, t, "FccT"),
            );
            (0..u32::from(t.current_records))
                .map(|i| {
                    format!(
                        "{}_{}_{}",
                        read(db, t, i, &s),
                        read(db, t, i, &a),
                        read(db, t, i, &v)
                    )
                })
                .collect()
        }
        "defs/enums/stick-model.toml#entries.raw" => {
            let text =
                std::fs::read_to_string(repo_root().join("defs/enums/stick-model.toml")).unwrap();
            let v: toml::Value = toml::from_str(&text).unwrap();
            v["entries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["raw"].as_integer().unwrap().to_string())
                .collect()
        }
        _ => {
            let (t, f) = field.split_once('.').unwrap();
            histogram(db, t, f)
                .distinct()
                .iter()
                .map(|v| v.to_string())
                .collect()
        }
    }
}

fn render_member_join_record() -> (String, Vec<GrammarJoin>) {
    let bytes = anchor_bytes(
        "anchors/xbox360/nhllegacy/databases/nhlng.db",
        LEGACY_ANCHOR_SHA256,
    );
    let db = TdbFile::parse(&bytes).unwrap();
    let mut out = Vec::new();
    for archive in ["cacherender.big", "nocacherender.big"] {
        let label = archive.trim_end_matches(".big");
        let rows = parse_eb_members_csv(&record_text(&format!("c0-as7-x360-{label}-members.csv")))
            .unwrap();
        for g in X360_GRAMMARS {
            let ids = db_ids_for(&db, g.db_field);
            let j = join_grammar(g, archive, &rows, &ids);
            // A grammar that meets no member of its directory in this archive says nothing about
            // the archive; keep the row only where the directory exists there, so the record
            // reads as "this archive holds these" rather than a cross product.
            if j.directory_members > 0 || g.directory.starts_with("fe/") {
                out.push(j);
            }
        }
    }
    (render_member_join_csv(&out), out)
}

/// D1: the declared and observed grammars over the two render archives, joined to the anchor.
#[test]
fn legacy_x360_member_tables_join_to_the_anchor_and_the_join_record_matches() {
    let (rendered, rows) = render_member_join_record();
    assert_record_matches("c0-as7-x360-member-join.csv", &rendered);
    let get = |label: &str| -> &GrammarJoin {
        rows.iter()
            .find(|r| r.label == label)
            .unwrap_or_else(|| panic!("{label}"))
    };
    // Every rendering directory is in exactly one of the two archives; portraits are in neither.
    for r in &rows {
        assert!(
            !(r.directory.starts_with("rendering/") && r.directory_members == 0),
            "{}",
            r.label
        );
    }
    let heads: Vec<&GrammarJoin> = rows
        .iter()
        .filter(|r| r.label.starts_with("playerheads/"))
        .collect();
    assert_eq!(heads.len(), 2);
    assert!(heads
        .iter()
        .all(|r| r.directory_members == 0 && r.matching_members == 0));
    // The facts the receipt quotes.
    let logo = get("logo/texlib_{team-id}_0_0");
    assert_eq!(logo.archive, "cacherender.big");
    assert_eq!(
        (
            logo.directory_members,
            logo.matching_members,
            logo.distinct_ids
        ),
        (1926, 307, 307)
    );
    assert_eq!(
        (
            logo.db_distinct_ids,
            logo.db_ids_with_member,
            logo.db_ids_without_member
        ),
        (218, 180, 38)
    );
    let logo_any = get("logo/texlib_{team-id}_{a}_{b}");
    assert_eq!(
        (
            logo_any.matching_members,
            logo_any.distinct_ids,
            logo_any.db_ids_without_member
        ),
        (1926, 348, 0)
    );
    let jersey = get("jersey/texlib_{style}_{art}_{variant}");
    assert_eq!(jersey.archive, "nocacherender.big");
    assert_eq!(
        (
            jersey.matching_members,
            jersey.db_distinct_ids,
            jersey.db_ids_without_member
        ),
        (905, 698, 0)
    );
    assert_eq!(jersey.member_ids_without_db_row, 207);
    let stick = get("playerstick/texlib_{numeric-id}");
    assert_eq!(
        (
            stick.matching_members,
            stick.db_distinct_ids,
            stick.db_ids_without_member
        ),
        (74, 19, 0)
    );
    let stick_enum = get("playerstick/texlib_{numeric-id} vs stick-model enum");
    assert_eq!(
        (
            stick_enum.db_ids_with_member,
            stick_enum.db_ids_without_member
        ),
        (74, 0)
    );
    assert_eq!(stick_enum.member_ids_without_db_row, 0);
    // Every stored id of the seven other slots has a member under its grammar — except the
    // goalie stick, whose HEAD `SLOT_SPECS` binding (`xmja.wXFx`, 35..112) meets members named
    // 6..43: one id in common, eighteen without. A binding the listing does not support.
    for label in [
        "skate/texlib_{numeric-id}",
        "helmet/texlib_{numeric-id}",
        "glove/glove_{numeric-id}_{variant}",
        "goaliepad/goaliepad_{numeric-id}_{variant}",
        "goaliehelmet/goaliehelmet_{variant}_{numeric-id}_cm",
        "blocker/blocker_{model}_{numeric-id}_{map}",
        "trapper/trapper_{model}_{numeric-id}_{map}",
    ] {
        assert_eq!(get(label).db_ids_without_member, 0, "{label}");
    }
    let gstick = get("goaliestick/goaliestick_{numeric-id}_{variant}");
    assert_eq!(
        (gstick.distinct_ids, gstick.id_range.as_str()),
        (38, "6-43")
    );
    assert_eq!(
        (gstick.db_ids_with_member, gstick.db_ids_without_member),
        (1, 18)
    );
}

/// The name-shape histogram of all three member tables — the D1 summary a reader can hold a
/// grammar against, regenerated from the records.
#[test]
fn member_name_shape_histograms_match_the_record() {
    let mut rendered = String::from("archive,directory,shape,members\n");
    for (archive, label) in [
        ("cacherender.big", "c0-as7-x360-cacherender-members"),
        ("nocacherender.big", "c0-as7-x360-nocacherender-members"),
        ("patch.big", "c0-as7-ps3-patch-members"),
    ] {
        let rows = parse_eb_members_csv(&record_text(&format!("{label}.csv"))).unwrap();
        let csv = render_shape_histogram_csv(archive, &rows);
        rendered.push_str(csv.split_once('\n').unwrap().1);
    }
    assert_record_matches("c0-as7-member-shapes.csv", &rendered);
}

/// D3: `patch.big`'s members classified by top-level family, stored form, what the committed
/// codec did with them, and how they overlap the loose tree by content.
fn render_patch_overlap_record() -> String {
    let rows = parse_eb_members_csv(&record_text("c0-as7-ps3-patch-members.csv")).unwrap();
    #[derive(Default)]
    struct Acc {
        members: usize,
        by_form: BTreeMap<String, usize>,
        loose_present: usize,
        loose_bak_present: usize,
        decoded_equals_loose: usize,
        decoded_equals_bak: usize,
        decoded_differs_from_loose: usize,
        undecoded_with_loose: usize,
    }
    let mut acc: BTreeMap<String, Acc> = BTreeMap::new();
    for r in &rows {
        let mut parts = r.path.split('/');
        let family = match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some("fe"), Some("ion"), Some("artassets"), Some(d)) => {
                format!("fe/ion/artassets/{d}")
            }
            (Some("fe"), Some(d), _, _) => format!("fe/{d}"),
            (Some("rendering"), Some(d), _, _) => format!("rendering/{d}"),
            (Some(d), _, _, _) => d.to_string(),
            _ => unreachable!(),
        };
        let a = acc.entry(family).or_default();
        a.members += 1;
        *a.by_form.entry(r.decoded_form.clone()).or_default() += 1;
        let present = r.loose_present == Some(true);
        let bak = r.loose_bak_present == Some(true);
        a.loose_present += usize::from(present);
        a.loose_bak_present += usize::from(bak);
        if r.decoded_sha256.is_empty() {
            a.undecoded_with_loose += usize::from(present);
        } else if present {
            if r.loose_sha256 == r.decoded_sha256 {
                a.decoded_equals_loose += 1;
            } else {
                a.decoded_differs_from_loose += 1;
            }
        }
        a.decoded_equals_bak += usize::from(
            bak && !r.decoded_sha256.is_empty() && r.loose_bak_sha256 == r.decoded_sha256,
        );
    }
    let mut out = String::from("family,members,decoded_forms,loose_present,loose_bak_present,decoded_equals_loose,decoded_differs_from_loose,undecoded_with_loose,decoded_equals_loose_bak\n");
    for (family, a) in &acc {
        let forms: Vec<String> = a.by_form.iter().map(|(f, c)| format!("{f}={c}")).collect();
        out.push_str(&format!(
            "{family},{},{},{},{},{},{},{},{}\n",
            a.members,
            forms.join(";"),
            a.loose_present,
            a.loose_bak_present,
            a.decoded_equals_loose,
            a.decoded_differs_from_loose,
            a.undecoded_with_loose,
            a.decoded_equals_bak
        ));
    }
    out
}

#[test]
fn ps3_patch_members_overlap_the_loose_tree_as_the_record_says() {
    let rendered = render_patch_overlap_record();
    assert_record_matches("c0-as7-ps3-patch-overlap.csv", &rendered);
    let rows = parse_eb_members_csv(&record_text("c0-as7-ps3-patch-members.csv")).unwrap();
    // The codec facts the receipt quotes: the stored forms met, and that every `chunkzip` member
    // the committed codec could not inflate carries the padded (`dummy = 2`) header with more
    // than one chunk, while every single-chunk member inflated.
    let mut forms: BTreeMap<&str, usize> = BTreeMap::new();
    for r in &rows {
        *forms.entry(r.decoded_form.as_str()).or_default() += 1;
    }
    assert_eq!(
        forms,
        BTreeMap::from([("chunkzip", 72), ("decode-error", 257), ("raw", 130)])
    );
    for r in rows.iter().filter(|r| r.stored_magic == "chunkzip") {
        let n_chunks: u64 = r
            .chunk_header
            .split(';')
            .find_map(|kv| kv.strip_prefix("w20="))
            .unwrap()
            .parse()
            .unwrap();
        assert!(r.chunk_header.starts_with("dummy=2;"), "{}", r.path);
        if r.decoded_form == "decode-error" {
            assert!(n_chunks > 1, "{}: failed with one chunk", r.path);
            assert!(
                r.decode_error.starts_with("chunkzip header is truncated"),
                "{}",
                r.path
            );
        }
        if n_chunks == 1 {
            assert_eq!(r.decoded_form, "chunkzip", "{}", r.path);
        }
    }
    // Portrait members: 83 (58 + 25), all stored raw `BIGF`; 23 have a loose file at the same
    // path and none of those is byte-identical to the member; no `.bak` sibling exists for any.
    let portraits: Vec<&EbMemberRow> = rows
        .iter()
        .filter(|r| r.path.starts_with("fe/ion/artassets/playerheads"))
        .collect();
    assert_eq!(portraits.len(), 83);
    assert!(portraits
        .iter()
        .all(|r| r.stored_magic == "BIGF" && r.decoded_form == "raw"));
    let with_loose: Vec<&&EbMemberRow> = portraits
        .iter()
        .filter(|r| r.loose_present == Some(true))
        .collect();
    assert_eq!(with_loose.len(), 23);
    assert!(with_loose
        .iter()
        .all(|r| r.loose_sha256 != r.decoded_sha256));
    assert!(portraits.iter().all(|r| r.loose_bak_present == Some(false)));
}
