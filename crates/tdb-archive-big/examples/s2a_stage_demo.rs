//! `s2a_stage_demo` — package S2-A: plan and stage entry replacements for one
//! `EB\0\3` archive with 16-byte rows (the PS3 NHL Legacy `patch.big`
//! layout) into a **new** file, then verify it. The source is only read.
//!
//! ```sh
//! cargo run -p tdb-archive-big --example s2a_stage_demo -- \
//!     --source <path/to/patch.big> --out <new-file> \
//!     [--edit <index>:noop] [--edit <index>:recompress] [--edit <index>:grow[:<bytes>]] ...
//! ```
//!
//! Edit modes (each names one entry by file-table index):
//! * `noop` — the entry's own stored bytes: the plan must be a no-op.
//! * `recompress` — decode the `chunkzip` member and re-encode it with this
//!   crate's single-chunk encoder; content identical, stored bytes differ.
//! * `grow[:n]` — a raw member with `n` (default 4096) space bytes appended to
//!   its content: forces append-and-retarget on a raw entry.
//!
//! Prints the plan, the stage report and the verification counts. This is a
//! demonstration of staging; nothing here installs, and the output's
//! `deploy_blockers` say whether it could be.

use std::path::PathBuf;

use tdb_archive_big::eb::{decode_payload, Payload};
use tdb_archive_big::stage::{
    hex, plan, stage_file, BoundSource, ExpectedDecoded, Placement, Replacement, StoredForm,
};
use tdb_archive_codec::compress_chunkzip;

fn main() {
    let mut source: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut edits: Vec<(usize, String, usize)> = Vec::new();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut v = || it.next().unwrap_or_else(|| panic!("{a} needs a value"));
        match a.as_str() {
            "--source" => source = Some(PathBuf::from(v())),
            "--out" => out = Some(PathBuf::from(v())),
            "--edit" => {
                let spec = v();
                let mut parts = spec.split(':');
                let index: usize = parts.next().unwrap().parse().expect("index");
                let mode = parts.next().unwrap_or("noop").to_owned();
                let n: usize = parts
                    .next()
                    .map(|s| s.parse().expect("bytes"))
                    .unwrap_or(4096);
                edits.push((index, mode, n));
            }
            other => panic!("unknown argument {other}"),
        }
    }
    let source = source.expect("--source");
    let out = out.expect("--out");

    let mut f = std::fs::File::open(&source).expect("open source");
    let bound = BoundSource::bind(&mut f).expect("bind");
    let l = &bound.layout;
    println!(
        "source: {} bytes, sha256 {}",
        bound.identity.len,
        hex(&bound.identity.sha256)
    );
    println!(
        "layout: version 0x{:08x}, {}-byte rows, {} entries, data_start {}, path-hash verified {}, word_4 zero {}, padding zero {}, table order = offset order {}",
        l.version, l.row_len, l.file_count, l.data_start, l.path_hash_verified, l.word_4_all_zero, l.padding_all_zero, l.table_order_is_offset_order
    );
    println!("opaque regions: {:?}", l.opaque_regions);
    println!("deploy blockers: {:?}", l.deploy_blockers);

    let mut reqs = Vec::new();
    for (index, mode, n) in edits {
        let e = bound.entries.get(index).expect("entry index");
        let stored = read_span(&mut f, e.offset, e.size);
        let plain = match decode_payload(stored.clone()).expect("decode") {
            Payload::Data(d) => d,
            Payload::UnsupportedLzx => panic!("entry {index} is chunklzx"),
        };
        let (new_stored, expected) = match mode.as_str() {
            "noop" => (stored.clone(), ExpectedDecoded::of(&plain)),
            "recompress" => {
                assert_eq!(
                    e.stored_form,
                    StoredForm::ChunkZip,
                    "recompress needs a chunkzip member"
                );
                (compress_chunkzip(&plain), ExpectedDecoded::of(&plain))
            }
            "grow" => {
                assert_eq!(
                    e.stored_form,
                    StoredForm::Raw,
                    "grow demo needs a raw member"
                );
                let mut bigger = plain.clone();
                bigger.extend(std::iter::repeat_n(b' ', n));
                (bigger.clone(), ExpectedDecoded::of(&bigger))
            }
            other => panic!("unknown mode {other}"),
        };
        println!(
            "edit {index} `{}` ({:?}, stored {} bytes at {}, slot end {}, sha256 {}): {mode} -> {} new stored bytes",
            e.path, e.stored_form, e.size, e.offset, e.slot_end, hex(&e.stored_sha256), new_stored.len()
        );
        reqs.push(Replacement {
            entry_index: index,
            expected_path: e.path.clone(),
            expected_stored_sha256: e.stored_sha256,
            new_stored,
            expected_decoded: expected,
        });
    }

    let p = plan(&bound, reqs).expect("plan");
    println!(
        "plan: original_eof {}, new_eof {}, noop {}, header edit {:?}",
        p.original_eof,
        p.new_eof,
        p.is_noop(),
        p.header_edit
    );
    for e in &p.edits {
        let placement = match &e.placement {
            Placement::NoOp => "no-op".to_owned(),
            Placement::FitInSlot { offset, slot_end } => {
                format!("fit-in-slot @ {offset} (slot end {slot_end})")
            }
            Placement::AppendAndRetarget { new_offset } => {
                format!("append-and-retarget @ {new_offset}")
            }
        };
        println!(
            "  entry {} `{}`: {placement}; old {}+{} -> new size {}; written {:?}",
            e.entry_index, e.path, e.old_offset, e.old_size, e.new_size, e.written_ranges
        );
        if let Some(r) = &e.row_edit {
            println!(
                "    row @ {}: {} -> {}",
                r.row_offset,
                r.old_row
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
                r.new_row
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
        }
    }
    println!("changed ranges: {:?}", p.changed_ranges);

    let (report, v) = stage_file(&bound, &p, &source, &out).expect("stage + verify");
    println!(
        "staged: {} -> {} bytes, output sha256 {}",
        report.original_eof,
        report.new_eof,
        hex(&report.output.sha256)
    );
    println!(
        "verified: {} entries reparsed, {} changed entries decoded and matched, {} unchanged rows compared, {} untouched bytes compared",
        v.entries_reparsed, v.changed_entries_decoded, v.unchanged_rows_compared, v.untouched_bytes_compared
    );
    println!("deploy blockers carried: {:?}", report.deploy_blockers);
    println!(
        "qualification: {:?} (deployable: {})",
        report.qualification.state,
        report.deployable()
    );
    for i in &report.qualification.items {
        println!("  [{:?}] {} — {}", i.status, i.subject, i.note);
    }
}

fn read_span(f: &mut std::fs::File, offset: u64, size: u32) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    f.seek(SeekFrom::Start(offset)).unwrap();
    let mut v = vec![0u8; size as usize];
    f.read_exact(&mut v).unwrap();
    v
}
