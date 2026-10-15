//! S2-A synthetic cases: layout binding, placement, streamed staging and
//! verification, and the malformed / conflict / truncation / tamper paths.

use super::*;
use crate::eb::{EbArchive, EbHeader};
use crate::eb::{FILE_TABLE_OFFSET as TABLE_OFF, MAGIC_EB};
use std::io::Cursor;
use tdb_archive_codec::compress_chunkzip;

// ----------------------------------------------------------------- builders

/// A synthetic archive in the qualified layout: version `0x0000_0400`,
/// 16-byte rows with `word_4 = 0` and `+12 = path_hash(path)`, payloads in
/// table order each padded with zeros to the next 16-byte line, and
/// `archive_size` = padded end = file length. `gap_after` inserts extra zero
/// lines after the given entry (so a slot can be wider than one line).
struct Synth {
    version: u32,
    dirs: Vec<&'static str>,
    files: Vec<(u16, &'static str, Vec<u8>)>,
    extra_lines_after: Vec<(usize, u64)>,
    /// Leave the opaque header words (0x10, 0x20..0x24) zero.
    zero_opaque: bool,
}

impl Synth {
    fn new() -> Self {
        Self {
            version: QUALIFIED_VERSION,
            dirs: vec!["rendering/blocker", "fe/loc"],
            files: vec![
                (
                    0,
                    "blocker_0_82_0_dm.rpsgl",
                    compress_chunkzip(&noise(3000, 1)),
                ),
                (
                    0,
                    "blocker_0_85_0_tm.rpsgl",
                    compress_chunkzip(&noise(700, 2)),
                ),
                (1, "nhl_eng_us-meta.xml", b"<meta/>raw-bytes".to_vec()),
                (1, "na.ini", b"1".to_vec()),
            ],
            extra_lines_after: Vec::new(),
            zero_opaque: false,
        }
    }

    fn build(&self) -> Vec<u8> {
        let row_len = crate::eb::entry_size_for(self.version);
        let file_count = self.files.len();
        let name_slot = self
            .files
            .iter()
            .map(|(_, n, _)| n.len() + 3)
            .max()
            .unwrap_or(3);
        let dir_slot = self.dirs.iter().map(|d| d.len() + 1).max().unwrap_or(1);
        // Real archives leave a zero gap between the file table and the name table;
        // reproduce one line of it so the opaque-region copy is exercised.
        let name_table_offset = TABLE_OFF + file_count * row_len + 16;
        let name_end = name_table_offset + file_count * name_slot;
        let dir_start = (name_end + 15) & !15;
        let table_end = dir_start + self.dirs.len() * dir_slot;
        let data_start = (table_end + 15) & !15;

        let mut offsets = Vec::new();
        let mut cursor = data_start as u64;
        for (i, (_, _, body)) in self.files.iter().enumerate() {
            offsets.push(cursor);
            cursor = (cursor + body.len() as u64 + 15) & !15;
            for &(after, lines) in &self.extra_lines_after {
                if after == i {
                    cursor += 16 * lines;
                }
            }
        }
        let archive_size = cursor;

        let mut out = vec![0u8; archive_size as usize];
        out[0..4].copy_from_slice(MAGIC_EB);
        out[0x04..0x08].copy_from_slice(&(file_count as u32).to_be_bytes());
        out[0x08..0x0C].copy_from_slice(&self.version.to_be_bytes());
        out[0x0C..0x10].copy_from_slice(&(name_table_offset as u32).to_be_bytes());
        if !self.zero_opaque {
            out[0x10..0x14].copy_from_slice(&0x4242u32.to_be_bytes()); // opaque, nonzero
        }
        out[0x14] = name_slot as u8;
        out[0x15] = dir_slot as u8;
        out[0x16..0x18].copy_from_slice(&(self.dirs.len() as u16).to_be_bytes());
        out[0x18..0x20].copy_from_slice(&archive_size.to_be_bytes());
        if !self.zero_opaque {
            out[0x20..0x24].copy_from_slice(&0x1234_5678u32.to_be_bytes()); // opaque, nonzero
        }
        for (i, (dir_index, name, body)) in self.files.iter().enumerate() {
            let e = TABLE_OFF + i * row_len;
            out[e..e + 4].copy_from_slice(&((offsets[i] >> 4) as u32).to_be_bytes());
            out[e + 8..e + 12].copy_from_slice(&(body.len() as u32).to_be_bytes());
            let path = format!("{}/{}", self.dirs[*dir_index as usize], name);
            if row_len == ROW_LEN_PS4 {
                // PS4 shape: an opaque path-derived sort key at +12, the path hash at +16.
                out[e + 12..e + 16].copy_from_slice(&(0x0100_0000u32 + i as u32).to_be_bytes());
                out[e + 16..e + 20].copy_from_slice(&path_hash(&path).to_be_bytes());
            } else {
                out[e + 12..e + 16].copy_from_slice(&path_hash(&path).to_be_bytes());
            }
            let n = name_table_offset + i * name_slot;
            out[n..n + 2].copy_from_slice(&dir_index.to_be_bytes());
            out[n + 2..n + 2 + name.len()].copy_from_slice(name.as_bytes());
            let o = offsets[i] as usize;
            out[o..o + body.len()].copy_from_slice(body);
        }
        for (i, dir) in self.dirs.iter().enumerate() {
            let d = dir_start + i * dir_slot;
            out[d..d + dir.len()].copy_from_slice(dir.as_bytes());
        }
        out
    }
}

fn sha(bytes: &[u8]) -> Sha256Bytes {
    Sha256::digest(bytes).into()
}

fn noise(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed | 1;
    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as u8
        })
        .collect()
}

fn bind(bytes: &[u8]) -> BoundSource {
    BoundSource::bind(&mut Cursor::new(bytes.to_vec())).unwrap()
}

fn replacement(src: &BoundSource, index: usize, plain: &[u8], wrap: bool) -> Replacement {
    let e = &src.entries[index];
    Replacement {
        entry_index: index,
        expected_path: e.path.clone(),
        expected_stored_sha256: e.stored_sha256,
        new_stored: if wrap {
            compress_chunkzip(plain)
        } else {
            plain.to_vec()
        },
        expected_decoded: ExpectedDecoded::of(plain),
    }
}

/// Stage into memory and verify; returns the output bytes and both reports.
fn stage_verify(
    src_bytes: &[u8],
    source: &BoundSource,
    plan: &Plan,
) -> (Vec<u8>, StageReport, VerificationReport) {
    let mut src = Cursor::new(src_bytes.to_vec());
    let mut out = Vec::new();
    let report = stage(source, plan, &mut src, &mut out).unwrap();
    let v = verify(
        source,
        plan,
        &report,
        &mut Cursor::new(out.clone()),
        &mut Cursor::new(src_bytes.to_vec()),
    )
    .unwrap();
    (out, report, v)
}

fn read_entry(bytes: &[u8], off: u64, size: u32) -> &[u8] {
    &bytes[off as usize..off as usize + size as usize]
}

// ----------------------------------------------------------------- binding

#[test]
fn bind_reports_identity_entries_and_layout_facts() {
    let a = Synth::new().build();
    let s = bind(&a);
    assert_eq!(s.identity.len, a.len() as u64);
    assert_eq!(s.identity.sha256, sha(&a));
    assert_eq!(s.entries.len(), 4);
    assert_eq!(
        s.entries[0].path,
        "rendering/blocker/blocker_0_82_0_dm.rpsgl"
    );
    assert_eq!(s.entries[0].stored_form, StoredForm::ChunkZip);
    assert_eq!(s.entries[2].stored_form, StoredForm::Raw);
    for e in &s.entries {
        assert_eq!(e.stored_sha256, sha(read_entry(&a, e.offset, e.size)));
        assert_eq!(e.word_12, path_hash(&e.path));
        assert!(!e.shared_span);
        assert_eq!(e.slot_end, (e.offset + e.size as u64 + 15) & !15);
    }
    let l = &s.layout;
    assert!(l.path_hash_verified && l.word_4_all_zero && l.padding_all_zero);
    assert!(l.table_order_is_offset_order && l.archive_size_is_file_len);
    assert!(l.deploy_blockers.is_empty());
    assert_eq!(l.data_start, (s.header.table_end() as u64 + 15) & !15);
    // The zero line between the file table and the name table is reported opaque.
    assert!(l
        .opaque_regions
        .contains(&((TABLE_OFF + 4 * 16) as u64..(TABLE_OFF + 4 * 16 + 16) as u64)));
    assert_eq!(s.prefix.len() as u64, l.data_start);
}

/// The 20-byte-row PS4 layout binds: the `+16` word is the path hash, the `+12` word is
/// preserved as an *unresolved* item (its formula is unknown), so the layout is never
/// deployable-qualified; an unknown version is still refused.
#[test]
fn bind_accepts_the_20_byte_row_variant_with_the_sort_key_unresolved() {
    let mut synth = Synth::new();
    synth.version = QUALIFIED_VERSION_PS4;
    let a = synth.build();
    let s = BoundSource::bind(&mut Cursor::new(a)).unwrap();
    assert_eq!(s.layout.row_len, ROW_LEN_PS4);
    assert!(s.layout.path_hash_verified && s.layout.word_4_all_zero);
    assert!(s.layout.deploy_blockers.is_empty());
    for e in &s.entries {
        assert_eq!(e.word_16, Some(path_hash(&e.path)));
        assert_eq!(e.word_12, 0x0100_0000 + e.index as u32);
    }
    let sort_key = s
        .layout
        .qualification_items
        .iter()
        .find(|i| i.subject == "row +12 word (PS4 sort key)")
        .expect("sort-key item");
    assert_eq!(sort_key.status, QualificationStatus::Unresolved);
    assert!(!DeploymentQualification::from_items(s.layout.qualification_items.clone()).is_qualified());

    let mut unknown = Synth::new().build();
    unknown[0x08..0x0C].copy_from_slice(&0x0022_0400u32.to_be_bytes());
    assert!(BoundSource::bind(&mut Cursor::new(unknown)).is_err());
}

/// A 20-byte-row archive stages exactly like the 16-byte one: no-op, fit-in-slot,
/// append-and-retarget, unchanged rows (both hash words) byte-for-byte, and verification.
#[test]
fn ps4_rows_stage_fit_and_append_and_verify() {
    let mut synth = Synth::new();
    synth.version = QUALIFIED_VERSION_PS4;
    let a = synth.build();
    let s = bind(&a);

    // No-op.
    let same = Replacement {
        entry_index: 2,
        expected_path: s.entries[2].path.clone(),
        expected_stored_sha256: s.entries[2].stored_sha256,
        new_stored: b"<meta/>raw-bytes".to_vec(),
        expected_decoded: ExpectedDecoded::of(b"<meta/>raw-bytes"),
    };
    let p = plan(&s, vec![same]).unwrap();
    assert!(p.is_noop());
    let (out, report, _) = stage_verify(&a, &s, &p);
    assert_eq!(out, a, "a no-op reproduces the source byte for byte");
    assert!(!report.deployable(), "never deployable-qualified: the +12 word is unresolved");

    // Shrink in place and grow past the line (append).
    let shrink = Replacement {
        entry_index: 2,
        expected_path: s.entries[2].path.clone(),
        expected_stored_sha256: s.entries[2].stored_sha256,
        new_stored: b"<m/>".to_vec(),
        expected_decoded: ExpectedDecoded::of(b"<m/>"),
    };
    let big = noise(5000, 9);
    let grow = Replacement {
        entry_index: 3,
        expected_path: s.entries[3].path.clone(),
        expected_stored_sha256: s.entries[3].stored_sha256,
        new_stored: big.clone(),
        expected_decoded: ExpectedDecoded::of(&big),
    };
    let p = plan(&s, vec![shrink, grow]).unwrap();
    assert!(matches!(p.edits[0].placement, Placement::FitInSlot { .. }));
    assert!(matches!(p.edits[1].placement, Placement::AppendAndRetarget { .. }));
    let row = p.edits[1].row_edit.as_ref().unwrap();
    assert_eq!(row.old_row.len, ROW_LEN_PS4);
    assert_eq!(&row.new_row[12..20], &row.old_row[12..20], "both hash words preserved verbatim");
    assert_eq!(&row.new_row[4..8], &[0; 4]);
    let (out, report, v) = stage_verify(&a, &s, &p);
    assert_eq!(report.new_eof, out.len() as u64);
    assert!(v.entries_reparsed >= 4);
    // Re-parse the output: untouched entries keep their bytes and rows; replaced ones decode.
    let re = bind(&out);
    assert_eq!(re.layout.row_len, ROW_LEN_PS4);
    assert!(re.layout.path_hash_verified);
    for i in [0usize, 1] {
        assert_eq!(re.entries[i].stored_sha256, s.entries[i].stored_sha256);
        assert_eq!(re.entries[i].word_12, s.entries[i].word_12);
        assert_eq!(re.entries[i].word_16, s.entries[i].word_16);
    }
    assert_eq!(re.entries[2].size, 4);
    assert_eq!(re.entries[3].size, 5000);
    assert_eq!(read_entry(&out, re.entries[3].offset, 5000), &big[..]);
}

#[test]
fn bind_refuses_archive_size_and_span_disagreements() {
    let a = Synth::new().build();
    let mut b = a.clone();
    b.extend_from_slice(&[0u8; 16]); // longer than declared
    let err = BoundSource::bind(&mut Cursor::new(b)).unwrap_err();
    assert!(
        matches!(err, StageError::ArchiveSizeMismatch { .. }),
        "{err}"
    );

    let mut c = a.clone();
    let e = TABLE_OFF + 3 * 16;
    c[e + 8..e + 12].copy_from_slice(&0x10_0000u32.to_be_bytes()); // size past EOF
    let err = BoundSource::bind(&mut Cursor::new(c)).unwrap_err();
    assert!(
        matches!(err, StageError::SpanOutOfRange { index: 3, .. }),
        "{err}"
    );

    let mut d = a;
    d.truncate(0x20);
    assert!(BoundSource::bind(&mut Cursor::new(d)).is_err());
}

#[test]
fn bind_records_unknown_row_words_as_deploy_blockers_and_flags_shared_spans() {
    let a = Synth::new().build();
    // Entry 1's +12 word made wrong, entry 2's +4 word made nonzero.
    let mut b = a.clone();
    let e1 = TABLE_OFF + 16;
    b[e1 + 12..e1 + 16].copy_from_slice(&0xDEAD_BEEFu32.to_be_bytes());
    let e2 = TABLE_OFF + 2 * 16;
    b[e2 + 4..e2 + 8].copy_from_slice(&7u32.to_be_bytes());
    let s = bind(&b);
    assert!(!s.layout.path_hash_verified && !s.layout.word_4_all_zero);
    assert_eq!(
        s.layout.deploy_blockers.len(),
        2,
        "{:?}",
        s.layout.deploy_blockers
    );
    assert!(
        s.layout.deploy_blockers[0].contains("entry 1")
            && s.layout.deploy_blockers[0].contains("+12")
    );
    assert!(
        s.layout.deploy_blockers[1].contains("entry 2")
            && s.layout.deploy_blockers[1].contains("+4")
    );

    // Entry 3 pointed at entry 2's block: both spans are shared.
    let mut c = a.clone();
    let e3 = TABLE_OFF + 3 * 16;
    let block2 = c[e2..e2 + 4].to_vec();
    c[e3..e3 + 4].copy_from_slice(&block2);
    let s = bind(&c);
    assert!(s.entries[2].shared_span && s.entries[3].shared_span);
    assert!(!s.entries[0].shared_span && !s.entries[1].shared_span);
    assert!(
        s.layout.table_order_is_offset_order,
        "equal offsets are still non-decreasing"
    );

    // Nonzero padding after entry 0: that entry gets no slack, the others keep
    // theirs, the region is reported and preserved, and it is not a blocker.
    let mut d = a;
    let end = s.entries[0].offset + s.entries[0].size as u64; // unchanged entry 0
    d[end as usize] = 0xEE;
    let s = bind(&d);
    assert!(!s.layout.padding_all_zero);
    assert_eq!(s.entries[0].slot_end, end);
    assert!(s.entries[1].slot_end > s.entries[1].offset + s.entries[1].size as u64);
    assert_eq!(s.layout.nonzero_gap_regions, vec![end..end + 1]);
    assert!(s.layout.deploy_blockers.is_empty());
}

// ----------------------------------------------------------------- planning

#[test]
fn identical_bytes_are_a_noop_and_stage_reproduces_the_source_exactly() {
    let a = Synth::new().build();
    let s = bind(&a);
    let plain = noise(700, 2);
    let mut r = replacement(&s, 1, &plain, true);
    // The synthetic payload *is* compress_chunkzip(plain), so this is byte-identical.
    r.new_stored = read_entry(&a, s.entries[1].offset, s.entries[1].size).to_vec();
    let p = plan(&s, vec![r]).unwrap();
    assert!(p.is_noop());
    assert_eq!(p.edits[0].placement, Placement::NoOp);
    assert!(p.edits[0].row_edit.is_none() && p.header_edit.is_none());
    assert_eq!(p.new_eof, p.original_eof);

    let (out, report, v) = stage_verify(&a, &s, &p);
    assert_eq!(
        out, a,
        "no-op output is byte-identical, padding and EOF included"
    );
    assert_eq!(report.output, report.source);
    assert_eq!(v.changed_entries_decoded, 1);
    assert_eq!(v.unchanged_rows_compared, 3);
    assert_eq!(v.untouched_bytes_compared, a.len() as u64);
}

#[test]
fn smaller_payload_fits_in_slot_zero_fills_slack_and_moves_nothing() {
    let a = Synth::new().build();
    let s = bind(&a);
    let plain = b"tiny".to_vec();
    let p = plan(&s, vec![replacement(&s, 0, &plain, true)]).unwrap();
    let e = &p.edits[0];
    let old = &s.entries[0];
    assert!(matches!(e.placement, Placement::FitInSlot { offset, .. } if offset == old.offset));
    assert!(e.new_size < old.size);
    let row = e.row_edit.as_ref().unwrap();
    assert_eq!(&row.new_row[0..4], &row.old_row[0..4], "block unchanged");
    assert_eq!(
        &row.new_row[12..16],
        &row.old_row[12..16],
        "path hash unchanged"
    );
    assert_eq!(
        u32::from_be_bytes(row.new_row[8..12].try_into().unwrap()),
        e.new_size
    );
    assert!(p.header_edit.is_none() && p.new_eof == a.len() as u64);
    assert_eq!(
        p.changed_ranges,
        vec![
            row.row_offset..row.row_offset + 16,
            old.offset..old.offset + old.size as u64
        ]
    );

    let (out, _, v) = stage_verify(&a, &s, &p);
    assert_eq!(out.len(), a.len());
    assert_eq!(
        read_entry(&out, old.offset, e.new_size),
        &compress_chunkzip(&plain)[..]
    );
    let slack =
        &out[old.offset as usize + e.new_size as usize..(old.offset + old.size as u64) as usize];
    assert!(slack.iter().all(|&b| b == 0), "slack zero-filled");
    for other in &s.entries[1..] {
        assert_eq!(
            read_entry(&out, other.offset, other.size),
            read_entry(&a, other.offset, other.size)
        );
    }
    assert_eq!(v.changed_entries_decoded, 1);
}

#[test]
fn payload_may_grow_into_its_own_zero_padding_but_not_past_the_line() {
    let a = Synth::new().build();
    let s = bind(&a);
    let e3 = &s.entries[3]; // "1", one byte, in a 16-byte line
    let slack = e3.slot_end - e3.offset;
    assert_eq!(slack, 16);
    let fits = vec![b'x'; 16];
    let p = plan(&s, vec![replacement(&s, 3, &fits, false)]).unwrap();
    assert!(matches!(p.edits[0].placement, Placement::FitInSlot { .. }));
    let (out, ..) = stage_verify(&a, &s, &p);
    assert_eq!(out.len(), a.len());

    let too_big = vec![b'y'; 17];
    let p = plan(&s, vec![replacement(&s, 3, &too_big, false)]).unwrap();
    assert!(matches!(
        p.edits[0].placement,
        Placement::AppendAndRetarget { .. }
    ));
}

#[test]
fn growth_appends_at_the_next_line_retargets_the_row_and_leaves_old_bytes() {
    let a = Synth::new().build();
    let s = bind(&a);
    let old = s.entries[1].clone();
    let plain = noise(9000, 9);
    let p = plan(&s, vec![replacement(&s, 1, &plain, true)]).unwrap();
    let e = &p.edits[0];
    let new_offset = match e.placement {
        Placement::AppendAndRetarget { new_offset } => new_offset,
        ref other => panic!("{other:?}"),
    };
    assert_eq!(
        new_offset,
        a.len() as u64,
        "source EOF is 16-aligned, so the append starts there"
    );
    let row = e.row_edit.as_ref().unwrap();
    assert_eq!(
        u32::from_be_bytes(row.new_row[0..4].try_into().unwrap()) as u64,
        new_offset >> 4
    );
    assert_eq!(
        u32::from_be_bytes(row.new_row[8..12].try_into().unwrap()),
        e.new_size
    );
    assert_eq!(&row.new_row[4..8], &[0; 4]);
    assert_eq!(&row.new_row[12..16], &row.old_row[12..16]);
    let h = p.header_edit.as_ref().unwrap();
    assert_eq!(h.old_archive_size, a.len() as u64);
    assert_eq!(
        h.new_archive_size,
        (new_offset + e.new_size as u64 + 15) & !15
    );
    assert_eq!(p.new_eof, h.new_archive_size);

    let (out, report, v) = stage_verify(&a, &s, &p);
    assert_eq!(out.len() as u64, p.new_eof);
    assert_eq!(
        &out[..a.len()][old.offset as usize..(old.offset + old.size as u64) as usize],
        read_entry(&a, old.offset, old.size),
        "old payload left where it was"
    );
    assert_eq!(
        read_entry(&out, new_offset, e.new_size),
        &compress_chunkzip(&plain)[..]
    );
    assert!(out[(new_offset + e.new_size as u64) as usize..]
        .iter()
        .all(|&b| b == 0));
    for other in s.entries.iter().filter(|x| x.index != 1) {
        assert_eq!(
            read_entry(&out, other.offset, other.size),
            read_entry(&a, other.offset, other.size)
        );
    }
    // Everything before the original EOF except the row and header words is identical.
    let mut expect = a.clone();
    expect[0x18..0x20].copy_from_slice(&p.new_eof.to_be_bytes());
    let ro = row.row_offset as usize;
    expect[ro..ro + 16].copy_from_slice(&row.new_row);
    assert_eq!(&out[..a.len()], &expect[..]);
    assert_eq!(report.new_eof, p.new_eof);
    assert_eq!(v.untouched_bytes_compared, a.len() as u64 - 16 - 8);
}

#[test]
fn multiple_edits_coalesce_deterministically() {
    let a = Synth::new().build();
    let s = bind(&a);
    let small = b"s".to_vec();
    let big0 = noise(5000, 30);
    let big2 = noise(4000, 31);
    let reqs = || {
        vec![
            replacement(&s, 2, &big2, false),  // raw, grows
            replacement(&s, 0, &big0, true),   // chunkzip, grows
            replacement(&s, 3, &small, false), // fits
        ]
    };
    let p1 = plan(&s, reqs()).unwrap();
    let p2 = plan(&s, reqs()).unwrap();
    assert_eq!(p1.edits, p2.edits);
    assert_eq!(p1.changed_ranges, p2.changed_ranges);
    // Ascending entry order; appends laid out in that order after the source EOF.
    assert_eq!(
        p1.edits.iter().map(|e| e.entry_index).collect::<Vec<_>>(),
        vec![0, 2, 3]
    );
    let off0 = match p1.edits[0].placement {
        Placement::AppendAndRetarget { new_offset } => new_offset,
        _ => panic!(),
    };
    let off2 = match p1.edits[1].placement {
        Placement::AppendAndRetarget { new_offset } => new_offset,
        _ => panic!(),
    };
    assert_eq!(off0, a.len() as u64);
    assert_eq!(off2, (off0 + p1.edits[0].new_size as u64 + 15) & !15);
    assert!(matches!(p1.edits[2].placement, Placement::FitInSlot { .. }));
    assert_eq!(p1.new_eof, (off2 + p1.edits[1].new_size as u64 + 15) & !15);

    let (out, _, v) = stage_verify(&a, &s, &p1);
    assert_eq!(v.changed_entries_decoded, 3);
    assert_eq!(v.unchanged_rows_compared, 1);
    assert_eq!(read_entry(&out, off2, p1.edits[1].new_size), &big2[..]);
    let re = BoundSource::bind(&mut Cursor::new(out.clone())).unwrap();
    assert_eq!(
        re.entries[1], s.entries[1],
        "untouched entry binds identically"
    );
    assert!(re.layout.deploy_blockers.is_empty() && re.layout.path_hash_verified);
    // The moved entries' old bytes are still there, now as reported nonzero gaps,
    // and the output stays plannable: a later fit into entry 3 still works.
    let (o0, o2) = (s.entries[0].offset, s.entries[2].offset);
    assert_eq!(
        read_entry(&out, o0, s.entries[0].size),
        read_entry(&a, o0, s.entries[0].size)
    );
    assert!(re.layout.nonzero_gap_regions.iter().any(|g| g.start == o0));
    assert!(re.layout.nonzero_gap_regions.iter().any(|g| g.start == o2));
    let again = plan(&re, vec![replacement(&re, 3, b"t", false)]).unwrap();
    assert!(matches!(
        again.edits[0].placement,
        Placement::FitInSlot { .. }
    ));
}

#[test]
fn plan_refuses_ambiguous_and_conflicting_replacements() {
    let a = Synth::new().build();
    let s = bind(&a);
    let plain = noise(100, 4);

    let err = plan(
        &s,
        vec![
            replacement(&s, 0, &plain, true),
            replacement(&s, 0, &plain, true),
        ],
    )
    .unwrap_err();
    assert!(
        matches!(err, StageError::DuplicateReplacement { index: 0, .. }),
        "{err}"
    );

    let mut r = replacement(&s, 0, &plain, true);
    r.expected_path = "rendering/blocker/other.rpsgl".into();
    let err = plan(&s, vec![r]).unwrap_err();
    assert!(
        matches!(err, StageError::PathMismatch { index: 0, .. }),
        "{err}"
    );

    let mut r = replacement(&s, 0, &plain, true);
    r.expected_stored_sha256[0] ^= 1;
    let err = plan(&s, vec![r]).unwrap_err();
    assert!(
        matches!(err, StageError::StoredHashMismatch { index: 0, .. }),
        "{err}"
    );

    let mut r = replacement(&s, 0, &plain, true);
    r.entry_index = 9;
    let err = plan(&s, vec![r]).unwrap_err();
    assert!(
        matches!(err, StageError::NoSuchEntry { index: 9, count: 4 }),
        "{err}"
    );

    // A malformed chunklzx payload is refused by its decode, not by its form; a store-encoded one
    // (research 2026-09-21 handoff 06) plans like any other payload that decodes to the expectation.
    let mut r = replacement(&s, 0, &plain, true);
    r.new_stored = b"chunklzx\0\0\0\x02rest".to_vec();
    let err = plan(&s, vec![r]).unwrap_err();
    assert!(
        matches!(err, StageError::DecodeFailed { index: 0, .. }),
        "{err}"
    );
    let mut r = replacement(&s, 0, &plain, true);
    r.new_stored = tdb_archive_codec::compress_chunklzx_store(&plain).unwrap();
    assert!(plan(&s, vec![r]).is_ok(), "a store-encoded chunklzx payload that decodes to the expectation plans");

    // New bytes that do not decode to the expectation are refused before any output.
    let mut r = replacement(&s, 0, &plain, true);
    r.expected_decoded = ExpectedDecoded::of(b"something else");
    let err = plan(&s, vec![r]).unwrap_err();
    assert!(
        matches!(err, StageError::DecodedMismatch { index: 0, .. }),
        "{err}"
    );
    let mut r = replacement(&s, 0, &plain, true);
    r.new_stored.truncate(20); // a chunkzip header cut short
    let err = plan(&s, vec![r]).unwrap_err();
    assert!(
        matches!(err, StageError::DecodeFailed { index: 0, .. }),
        "{err}"
    );

    // A shared-span entry cannot be replaced.
    let mut c = a.clone();
    let e2 = TABLE_OFF + 2 * 16;
    let e3 = TABLE_OFF + 3 * 16;
    let block2 = c[e2..e2 + 4].to_vec();
    c[e3..e3 + 4].copy_from_slice(&block2);
    let s2 = bind(&c);
    let err = plan(&s2, vec![replacement(&s2, 3, &plain, false)]).unwrap_err();
    assert!(
        matches!(err, StageError::SharedSpan { index: 3, .. }),
        "{err}"
    );
    // ...but an unshared one in the same archive can.
    assert!(plan(&s2, vec![replacement(&s2, 0, &plain, true)]).is_ok());
}

/// Offsets beyond 4 GB and the block-word limit, on a hand-built bound source
/// (no multi-gigabyte file is written).
#[test]
fn append_offsets_beyond_4gb_are_planned_with_checked_arithmetic_and_refused_past_the_block_word() {
    let a = Synth::new().build();
    let mut s = bind(&a);
    let plain = noise(20_000, 5); // does not fit entry 0's slot

    // Pretend the archive ends just below 5 GiB (unaligned, to exercise the align step).
    let five_gib: u64 = 5 << 30;
    s.identity.len = five_gib - 7;
    let p = plan(&s, vec![replacement(&s, 0, &plain, true)]).unwrap();
    match p.edits[0].placement {
        Placement::AppendAndRetarget { new_offset } => {
            assert_eq!(new_offset, five_gib);
            assert_eq!(new_offset % 16, 0);
        }
        ref other => panic!("{other:?}"),
    }
    let row = p.edits[0].row_edit.as_ref().unwrap();
    assert_eq!(
        u32::from_be_bytes(row.new_row[0..4].try_into().unwrap()),
        (five_gib >> 4) as u32
    );
    assert!(p.new_eof > u32::MAX as u64);
    assert_eq!(p.header_edit.as_ref().unwrap().new_archive_size, p.new_eof);
    assert_eq!(
        p.changed_ranges.last().unwrap().start,
        five_gib - 7,
        "the pad from the old EOF is new bytes"
    );

    // Exactly at the limit is representable; one line past it is not.
    s.identity.len = MAX_BLOCK_OFFSET;
    let p = plan(&s, vec![replacement(&s, 0, &plain, true)]).unwrap();
    assert!(
        matches!(p.edits[0].placement, Placement::AppendAndRetarget { new_offset } if new_offset == MAX_BLOCK_OFFSET)
    );
    s.identity.len = MAX_BLOCK_OFFSET + 1;
    let err = plan(&s, vec![replacement(&s, 0, &plain, true)]).unwrap_err();
    assert!(
        matches!(err, StageError::OffsetNotRepresentable { index: 0, .. }),
        "{err}"
    );

    // Arithmetic that would wrap is an overflow error, never a wrapped offset.
    s.identity.len = u64::MAX - 3;
    let err = plan(&s, vec![replacement(&s, 0, &plain, true)]).unwrap_err();
    assert!(matches!(err, StageError::Overflow { .. }), "{err}");
}

// ----------------------------------------------------------------- staging failures

#[test]
fn stage_refuses_a_source_that_changed_after_binding() {
    let a = Synth::new().build();
    let s = bind(&a);
    let p = plan(&s, vec![replacement(&s, 1, &noise(50, 6), true)]).unwrap();

    // Same length, one payload byte differs: caught by the streamed hash.
    let mut changed = a.clone();
    let e2 = &s.entries[2];
    changed[e2.offset as usize] ^= 0x40;
    let mut out = Vec::new();
    let err = stage(&s, &p, &mut Cursor::new(changed), &mut out).unwrap_err();
    assert!(matches!(err, StageError::SourceChanged { .. }), "{err}");

    // Different length: caught before anything is read.
    let mut longer = a.clone();
    longer.extend_from_slice(&[0; 16]);
    let mut out = Vec::new();
    let err = stage(&s, &p, &mut Cursor::new(longer), &mut out).unwrap_err();
    assert!(matches!(err, StageError::SourceChanged { .. }), "{err}");
    assert!(out.is_empty());

    // A plan for another source is refused.
    let mut other = Synth::new();
    other.files[3].2 = b"2".to_vec();
    let ob = other.build();
    let os = bind(&ob);
    let err = stage(&os, &p, &mut Cursor::new(ob), &mut Vec::new()).unwrap_err();
    assert!(
        matches!(err, StageError::PlanSourceMismatch { .. }),
        "{err}"
    );
}

/// A sink that fails after `limit` bytes.
struct Truncating {
    limit: usize,
    written: Vec<u8>,
}
impl Write for Truncating {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written.len() + buf.len() > self.limit {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "disk full"));
        }
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_truncated_write_is_an_error_and_the_source_is_untouched() {
    let a = Synth::new().build();
    let s = bind(&a);
    let p = plan(&s, vec![replacement(&s, 0, &noise(8000, 7), true)]).unwrap();
    let src_bytes = a.clone();
    let mut src = Cursor::new(src_bytes);
    for limit in [0, 10, a.len() / 2, a.len() + 5] {
        let mut sink = Truncating {
            limit,
            written: Vec::new(),
        };
        src.set_position(0);
        let err = stage(&s, &p, &mut src, &mut sink).unwrap_err();
        assert!(matches!(err, StageError::Io(_)), "limit {limit}: {err}");
        assert!(sink.written.len() <= limit);
    }
    assert_eq!(src.into_inner(), a, "source bytes never change");
}

#[test]
fn verify_detects_tampering_in_changed_and_untouched_ranges() {
    let a = Synth::new().build();
    let s = bind(&a);
    let plain = noise(6000, 8);
    let p = plan(
        &s,
        vec![
            replacement(&s, 1, &plain, true),
            replacement(&s, 3, b"q", false),
        ],
    )
    .unwrap();
    let (out, report, _) = stage_verify(&a, &s, &p);
    let run = |bytes: Vec<u8>, rep: &StageReport| {
        verify(
            &s,
            &p,
            rep,
            &mut Cursor::new(bytes),
            &mut Cursor::new(a.clone()),
        )
    };

    // Any byte change is first caught by the whole-file hash...
    let mut t = out.clone();
    t[s.entries[2].offset as usize + 3] ^= 1;
    assert!(matches!(
        run(t, &report).unwrap_err(),
        StageError::OutputHashMismatch { .. }
    ));

    // ...so give the report the tampered hash to reach the structural checks.
    let with_hash = |bytes: &[u8]| StageReport {
        output: SourceIdentity {
            len: bytes.len() as u64,
            sha256: Sha256::digest(bytes).into(),
        },
        ..report.clone()
    };
    // Untouched payload byte.
    let mut t = out.clone();
    let at = s.entries[2].offset as usize + 3;
    t[at] ^= 1;
    let err = run(t.clone(), &with_hash(&t)).unwrap_err();
    assert!(
        matches!(err, StageError::UntouchedBytesDiffer { at: a2, .. } if a2 == at as u64),
        "{err}"
    );
    // Opaque header word.
    let mut t = out.clone();
    t[0x11] ^= 1;
    assert!(matches!(
        run(t.clone(), &with_hash(&t)).unwrap_err(),
        StageError::UntouchedBytesDiffer { at: 0x11, .. }
    ));
    // An untouched row.
    let mut t = out.clone();
    t[TABLE_OFF + 2 * 16 + 9] ^= 1;
    assert!(matches!(
        run(t.clone(), &with_hash(&t)).unwrap_err(),
        StageError::RowMismatch { index: 2, .. }
    ));
    // The appended payload.
    let new_off = match p.edits[0].placement {
        Placement::AppendAndRetarget { new_offset } => new_offset,
        _ => panic!(),
    };
    let mut t = out.clone();
    t[new_off as usize + 50] ^= 1;
    assert!(matches!(
        run(t.clone(), &with_hash(&t)).unwrap_err(),
        StageError::StoredMismatch { index: 1, .. }
    ));
    // The retargeted row.
    let mut t = out.clone();
    t[TABLE_OFF + 16 + 9] ^= 1;
    assert!(matches!(
        run(t.clone(), &with_hash(&t)).unwrap_err(),
        StageError::RowMismatch { index: 1, .. }
    ));
    // Header archive_size.
    let mut t = out.clone();
    t[0x1F] ^= 1;
    assert!(matches!(
        run(t.clone(), &with_hash(&t)).unwrap_err(),
        StageError::OutputArchiveSizeMismatch { .. }
    ));
    // Length.
    let mut t = out.clone();
    t.push(0);
    assert!(matches!(
        run(t.clone(), &with_hash(&t)).unwrap_err(),
        StageError::OutputLengthMismatch { .. }
    ));

    // A stored payload that still hashes right but decodes to the wrong content
    // cannot exist; a plan whose expectation is wrong is refused at plan time
    // (see the planning cases) and a payload rewritten in place trips the hash.
}

// ----------------------------------------------------------------- files

#[test]
fn stage_file_creates_a_new_file_refuses_an_existing_one_and_removes_a_failed_output() {
    let dir = std::env::temp_dir().join(format!("tdb-archive-big-stage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src_path = dir.join("source.big");
    let out_path = dir.join("staged.big");
    let a = Synth::new().build();
    std::fs::write(&src_path, &a).unwrap();
    let s = BoundSource::bind(&mut std::fs::File::open(&src_path).unwrap()).unwrap();
    let plain = noise(2500, 11);
    let p = plan(&s, vec![replacement(&s, 0, &plain, true)]).unwrap();

    let (report, v) = stage_file(&s, &p, &src_path, &out_path).unwrap();
    let out = std::fs::read(&out_path).unwrap();
    assert_eq!(out.len() as u64, report.output.len);
    assert_eq!(report.output.sha256, sha(&out));
    assert_eq!(v.changed_entries_decoded, 1);
    assert_eq!(std::fs::read(&src_path).unwrap(), a, "source untouched");

    // Existing output is refused and left alone.
    let err = stage_file(&s, &p, &src_path, &out_path).unwrap_err();
    assert!(matches!(err, StageError::Io(_)), "{err}");
    assert_eq!(std::fs::read(&out_path).unwrap(), out);

    // A source that changed since binding: the partial output is removed.
    let out2 = dir.join("staged2.big");
    let mut changed = a.clone();
    changed[s.entries[2].offset as usize] ^= 1;
    std::fs::write(&src_path, &changed).unwrap();
    let err = stage_file(&s, &p, &src_path, &out2).unwrap_err();
    assert!(matches!(err, StageError::SourceChanged { .. }), "{err}");
    assert!(!out2.exists(), "failed output removed");
    assert_eq!(
        std::fs::read(&src_path).unwrap(),
        changed,
        "source untouched on failure"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn range_helpers_normalize_and_complement() {
    assert_eq!(
        normalize(vec![5..7, 0..2, 6..9, 2..2, 9..9]),
        vec![0..2, 5..9]
    );
    assert_eq!(complement(&[0..2, 5..9], 12), vec![2..5, 9..12]);
    assert_eq!(complement(&[], 3), vec![0..3]);
    let one = std::slice::from_ref(&(0..3u64));
    assert_eq!(complement(one, 3), Vec::<Range<u64>>::new());
    let wide = std::slice::from_ref(&(2..40u64));
    assert_eq!(
        complement(wide, 10),
        std::slice::from_ref(&(0..2u64)).to_vec()
    );
}

#[test]
fn path_hash_is_djb2() {
    // Verified against the PS3 `patch.big` row words (C0-AS7 record, S2-A receipt).
    assert_eq!(path_hash("sku/na.ini"), 0xda05_8384);
    assert_eq!(path_hash("fe/credits/credits"), 0x71d4_39ca);
    assert_eq!(path_hash(""), 5381);
}

// ----------------------------------------------------------------- S2-A-R1: malformed table layouts

/// The review reproducer: 64 zero bytes with magic, file count 1, version
/// `0x400`, name-table offset 32 (inside the fixed file table), name slot 3,
/// dir slot 1, dir count 1, archive size 64. Before R1 this panicked inside
/// the parser; it must be an error from bind, from the parser, and from the
/// verification reparse — and never an allocation from the header's claims.
fn review_reproducer() -> Vec<u8> {
    let mut b = vec![0u8; 64];
    b[0..4].copy_from_slice(MAGIC_EB);
    b[0x04..0x08].copy_from_slice(&1u32.to_be_bytes());
    b[0x08..0x0C].copy_from_slice(&QUALIFIED_VERSION.to_be_bytes());
    b[0x0C..0x10].copy_from_slice(&32u32.to_be_bytes());
    b[0x14] = 3;
    b[0x15] = 1;
    b[0x16..0x18].copy_from_slice(&1u16.to_be_bytes());
    b[0x18..0x20].copy_from_slice(&64u64.to_be_bytes());
    b
}

#[test]
fn review_reproducer_is_rejected_as_overlapping_tables_not_a_panic() {
    let b = review_reproducer();
    let err = BoundSource::bind(&mut Cursor::new(b.clone())).unwrap_err();
    assert!(
        matches!(
            err,
            StageError::Parse(crate::eb::EbError::TablesOverlap {
                file_table_end: 64,
                name_table_offset: 32
            })
        ),
        "{err}"
    );
    // The shared parser refuses it too, for any prefix length.
    for n in [0x20, 49, 64] {
        assert!(EbArchive::parse(&b[..n]).is_err(), "prefix {n}");
    }
    let err = EbHeader::parse(&b).unwrap().table_layout(64).unwrap_err();
    assert!(matches!(err, crate::eb::EbError::TablesOverlap { .. }));
}

#[test]
fn truncated_file_table_and_header_claims_beyond_the_input_are_rejected_before_allocation() {
    let a = Synth::new().build();

    // File table longer than the file: 1 000 rows claimed, name table placed after
    // them (no overlap) — the tables end past the input.
    let mut b = a.clone();
    b[0x04..0x08].copy_from_slice(&1000u32.to_be_bytes());
    b[0x0C..0x10].copy_from_slice(&((TABLE_OFF + 1000 * 16) as u32).to_be_bytes());
    let err = BoundSource::bind(&mut Cursor::new(b)).unwrap_err();
    assert!(
        matches!(
            err,
            StageError::Parse(crate::eb::EbError::TruncatedTables { .. })
        ),
        "{err}"
    );

    // A file count that would have sized a multi-gigabyte prefix: refused by
    // arithmetic on the header alone (this test would not finish otherwise).
    let mut c = a.clone();
    c[0x04..0x08].copy_from_slice(&0x7FFF_FFFFu32.to_be_bytes());
    c[0x0C..0x10].copy_from_slice(&u32::MAX.to_be_bytes());
    let err = BoundSource::bind(&mut Cursor::new(c)).unwrap_err();
    assert!(
        matches!(
            err,
            StageError::Parse(
                crate::eb::EbError::TruncatedTables { .. }
                    | crate::eb::EbError::TablesOverlap { .. }
            )
        ),
        "{err}"
    );

    // Directory table claimed past the input.
    let mut d = a.clone();
    d[0x16..0x18].copy_from_slice(&u16::MAX.to_be_bytes());
    d[0x15] = 255;
    let err = BoundSource::bind(&mut Cursor::new(d)).unwrap_err();
    assert!(
        matches!(
            err,
            StageError::Parse(crate::eb::EbError::TruncatedTables { .. })
        ),
        "{err}"
    );

    // Tables fit but the payload region's first line does not.
    let mut e = a.clone();
    let s = bind(&a);
    e.truncate(s.layout.data_start as usize - 1);
    let new_len = e.len() as u64;
    e[0x18..0x20].copy_from_slice(&new_len.to_be_bytes());
    let err = BoundSource::bind(&mut Cursor::new(e)).unwrap_err();
    assert!(
        matches!(
            err,
            StageError::Parse(crate::eb::EbError::TruncatedTables { .. })
        ),
        "{err}"
    );
}

#[test]
fn overlapping_metadata_tables_are_rejected_on_an_otherwise_valid_archive() {
    let a = Synth::new().build();
    let s = bind(&a);
    let tables = s.header.table_layout(a.len() as u64).unwrap();
    assert_eq!(tables.file_table_end, (TABLE_OFF + 4 * 16) as u64);
    assert_eq!(tables.data_start, s.layout.data_start);

    // Name table starting one byte inside the file table.
    let mut b = a.clone();
    b[0x0C..0x10].copy_from_slice(&((TABLE_OFF + 4 * 16 - 1) as u32).to_be_bytes());
    let err = BoundSource::bind(&mut Cursor::new(b)).unwrap_err();
    assert!(
        matches!(
            err,
            StageError::Parse(crate::eb::EbError::TablesOverlap { .. })
        ),
        "{err}"
    );
    // Name table at offset 0.
    let mut c = a;
    c[0x0C..0x10].copy_from_slice(&0u32.to_be_bytes());
    assert!(matches!(
        BoundSource::bind(&mut Cursor::new(c)).unwrap_err(),
        StageError::Parse(crate::eb::EbError::TablesOverlap { .. })
    ));
}

#[test]
fn verification_reparse_refuses_a_malformed_staged_header() {
    let a = Synth::new().build();
    let s = bind(&a);
    let p = plan(&s, vec![replacement(&s, 1, &noise(50, 21), true)]).unwrap();
    let (out, report, _) = stage_verify(&a, &s, &p);
    let with_hash = |bytes: &[u8]| StageReport {
        output: SourceIdentity {
            len: bytes.len() as u64,
            sha256: sha(bytes),
        },
        ..report.clone()
    };
    // Overlapping tables (the reproducer's shape) in the output header.
    let mut t = out.clone();
    t[0x0C..0x10].copy_from_slice(&32u32.to_be_bytes());
    let err = verify(
        &s,
        &p,
        &with_hash(&t),
        &mut Cursor::new(t.clone()),
        &mut Cursor::new(a.clone()),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            StageError::Parse(crate::eb::EbError::TablesOverlap { .. })
        ),
        "{err}"
    );
    // Tables claimed far beyond the output.
    let mut t = out.clone();
    t[0x04..0x08].copy_from_slice(&0x7FFF_FFFFu32.to_be_bytes());
    t[0x0C..0x10].copy_from_slice(&u32::MAX.to_be_bytes());
    let err = verify(
        &s,
        &p,
        &with_hash(&t),
        &mut Cursor::new(t.clone()),
        &mut Cursor::new(a.clone()),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            StageError::Parse(
                crate::eb::EbError::TruncatedTables { .. }
                    | crate::eb::EbError::TablesOverlap { .. }
            )
        ),
        "{err}"
    );
}

// ----------------------------------------------------------------- S2-A-R2: qualification

fn item<'a>(q: &'a [QualificationItem], subject: &str) -> &'a QualificationItem {
    q.iter()
        .find(|i| i.subject == subject)
        .unwrap_or_else(|| panic!("no item `{subject}` in {q:#?}"))
}

#[test]
fn bind_reports_every_preserved_unknown_as_a_qualification_item() {
    let a = Synth::new().build(); // nonzero 0x10 and 0x20 words
    let s = bind(&a);
    let q = &s.layout.qualification_items;
    assert_eq!(
        item(q, "row +12 word (path hash)").status,
        QualificationStatus::Resolved
    );
    assert_eq!(item(q, "row +4 word").status, QualificationStatus::Resolved);
    assert_eq!(
        item(q, "header word 0x10").status,
        QualificationStatus::Unresolved
    );
    assert!(item(q, "header word 0x10").note.contains("16962")); // 0x4242
    assert_eq!(
        item(q, "header bytes 0x20..0x30").status,
        QualificationStatus::Unresolved
    );
    assert_eq!(
        item(q, "table gaps and alignment pads").status,
        QualificationStatus::Resolved
    );
    assert_eq!(
        item(q, "payload-region gaps").status,
        QualificationStatus::Resolved
    );
    assert_eq!(
        item(q, "header archive_size").status,
        QualificationStatus::Resolved
    );
    assert!(
        s.layout.deploy_blockers.is_empty(),
        "unknowns are not contradictions"
    );

    // A layout contradiction is Blocked and a blocker at once.
    let mut b = a.clone();
    let e1 = TABLE_OFF + 16;
    b[e1 + 12..e1 + 16].copy_from_slice(&0xDEAD_BEEFu32.to_be_bytes());
    let s = bind(&b);
    assert_eq!(
        item(&s.layout.qualification_items, "row +12 word (path hash)").status,
        QualificationStatus::Blocked
    );
    assert_eq!(s.layout.deploy_blockers.len(), 1);

    // Zero opaque words resolve those items — and nothing else changes.
    let mut z = Synth::new();
    z.zero_opaque = true;
    let s = bind(&z.build());
    assert_eq!(
        item(&s.layout.qualification_items, "header word 0x10").status,
        QualificationStatus::Resolved
    );
    assert_eq!(
        item(&s.layout.qualification_items, "header bytes 0x20..0x30").status,
        QualificationStatus::Resolved
    );
    assert!(s
        .layout
        .qualification_items
        .iter()
        .all(|i| i.status == QualificationStatus::Resolved));
}

#[test]
fn unresolved_qualification_survives_plan_stage_and_handoff_and_blocks_deployment() {
    // Even a layout with every structural item resolved is unqualified: no
    // staged output has been game-verified, and every replacement is unproven.
    let mut z = Synth::new();
    z.zero_opaque = true;
    let a = z.build();
    let s = bind(&a);

    // No-op only: layout items + native loading; no per-entry item.
    let mut r = replacement(&s, 1, &noise(700, 2), true);
    r.new_stored = read_entry(&a, s.entries[1].offset, s.entries[1].size).to_vec();
    let p = plan(&s, vec![r]).unwrap();
    assert!(p.is_noop());
    assert_eq!(p.qualification.state, QualificationState::Unqualified);
    assert!(!p
        .qualification
        .items
        .iter()
        .any(|i| i.subject.contains("replacement")));
    let loader = item(&p.qualification.items, "native loader acceptance");
    assert_eq!(loader.status, QualificationStatus::Unresolved);
    let (_, report, v) = stage_verify(&a, &s, &p);
    assert_eq!(report.qualification, p.qualification);
    assert!(!report.deployable(), "verified is not qualified");
    assert_eq!(v.changed_entries_decoded, 1);

    // A single-chunk recompression (fit) and a raw growth (append): one
    // unresolved item each, the chunkzip one naming the framing difference.
    let p = plan(
        &s,
        vec![
            replacement(&s, 1, b"small", true),
            replacement(&s, 2, &noise(3000, 22), false),
        ],
    )
    .unwrap();
    let e1 = item(
        &p.qualification.items,
        &format!("entry 1 `{}`: replacement", s.entries[1].path),
    );
    assert_eq!(e1.status, QualificationStatus::Unresolved);
    assert!(e1.note.starts_with("fit-in-slot"), "{}", e1.note);
    assert!(
        e1.note.contains("chunk_size") && e1.note.contains("not qualified"),
        "{}",
        e1.note
    );
    let e2 = item(
        &p.qualification.items,
        &format!("entry 2 `{}`: replacement", s.entries[2].path),
    );
    assert!(e2.note.starts_with("append-and-retarget"), "{}", e2.note);
    assert!(!e2.note.contains("chunkzip"), "{}", e2.note);
    let open: Vec<_> = p
        .qualification
        .open_items()
        .map(|i| i.subject.clone())
        .collect();
    assert_eq!(open.len(), 3, "{open:?}");
    let (_, report, _) = stage_verify(&a, &s, &p);
    assert_eq!(report.qualification, p.qualification);
    assert!(report.deploy_blockers.is_empty() && !report.deployable());

    // A native-framed chunkzip payload is still unresolved (provenance).
    let mut native = replacement(&s, 0, &noise(3000, 1), true);
    let mut framed = native.new_stored.clone();
    framed[16..20].copy_from_slice(&NATIVE_CHUNK_SIZE.to_be_bytes());
    native.new_stored = framed;
    let p = plan(&s, vec![native]).unwrap();
    let e0 = item(
        &p.qualification.items,
        &format!("entry 0 `{}`: replacement", s.entries[0].path),
    );
    assert!(e0.note.contains("framed like native") && e0.status == QualificationStatus::Unresolved);

    // A contradiction is both a blocker and a Blocked item after staging.
    let mut b = a.clone();
    let e3 = TABLE_OFF + 3 * 16;
    b[e3 + 4..e3 + 8].copy_from_slice(&5u32.to_be_bytes());
    let s = bind(&b);
    let p = plan(&s, vec![replacement(&s, 1, b"x", true)]).unwrap();
    let (_, report, _) = stage_verify(&b, &s, &p);
    assert_eq!(report.deploy_blockers.len(), 1);
    assert_eq!(
        item(&report.qualification.items, "row +4 word").status,
        QualificationStatus::Blocked
    );
    assert!(!report.deployable());
}

#[test]
fn qualified_requires_every_item_resolved() {
    use QualificationStatus::*;
    let mk = |st| QualificationItem::new("x", st, "");
    assert!(DeploymentQualification::from_items(vec![]).is_qualified());
    assert!(DeploymentQualification::from_items(vec![mk(Resolved), mk(Resolved)]).is_qualified());
    for bad in [Unresolved, Blocked] {
        let q = DeploymentQualification::from_items(vec![mk(Resolved), mk(bad)]);
        assert_eq!(q.state, QualificationState::Unqualified);
        assert_eq!(q.open_items().count(), 1);
    }
    let report = StageReport {
        source: SourceIdentity {
            len: 0,
            sha256: [0; 32],
        },
        output: SourceIdentity {
            len: 0,
            sha256: [0; 32],
        },
        original_eof: 0,
        new_eof: 0,
        changed_ranges: vec![],
        deploy_blockers: vec!["contradiction".into()],
        qualification: DeploymentQualification::from_items(vec![]),
    };
    assert!(!report.deployable(), "a blocker alone blocks deployment");
}

// ----------------------------------------------------------------- EB-V1: PS4 NHL 15 layout

/// The PS4 NHL 15 layout (`0x0010_0400`) binds and stages exactly like the 16-byte Legacy layout
/// when its `archive_size` equals the file length.
#[test]
fn nhl15_layout_binds_with_16_byte_rows_and_stages_a_same_length_replacement() {
    let mut synth = Synth::new();
    synth.version = QUALIFIED_VERSION_PS4_16;
    let a = synth.build();
    let s = bind(&a);
    assert_eq!(s.layout.version, QUALIFIED_VERSION_PS4_16);
    assert_eq!(s.layout.row_len, ROW_LEN);
    assert!(s.layout.path_hash_verified && s.layout.word_4_all_zero);
    assert!(s.layout.archive_size_is_file_len);
    assert!(s.layout.deploy_blockers.is_empty());
    for e in &s.entries {
        assert_eq!(e.word_12, path_hash(&e.path));
        assert_eq!(e.word_16, None);
    }
    let size_item = s
        .layout
        .qualification_items
        .iter()
        .find(|i| i.subject == "header archive_size")
        .unwrap();
    assert_eq!(size_item.status, QualificationStatus::Resolved);
    // Same-length raw replacement of the raw member (entry 2, 16 bytes).
    let new = b"<meta/>NEW-BYTES".to_vec();
    assert_eq!(new.len(), s.entries[2].size as usize);
    let plan = plan(&s, vec![replacement(&s, 2, &new, false)]).unwrap();
    assert!(plan.header_edit.is_none());
    assert_eq!(plan.expected_archive_size, a.len() as u64);
    let (out, _, v) = stage_verify(&a, &s, &plan);
    assert_eq!(out.len(), a.len());
    assert_eq!(read_entry(&out, s.entries[2].offset, 16), &new[..]);
    assert_eq!(v.entries_reparsed, 4);
    assert!(plan.qualification.open_items().count() > 0, "never deployable-qualified by structure alone");
}

/// Three of the 17 NHL 15 archives declare an `archive_size` *below* the file length while their
/// entries end exactly at the file length. That is bound and recorded as an unresolved item, the
/// word is preserved verbatim through a same-length replacement, and any plan that would change
/// the file length is refused. The same discrepancy on the Legacy layout stays a refusal.
#[test]
fn nhl15_declared_archive_size_below_the_length_is_recorded_preserved_and_never_grown() {
    let mut synth = Synth::new();
    synth.version = QUALIFIED_VERSION_PS4_16;
    let mut a = synth.build();
    let declared = a.len() as u64 - 4096;
    a[0x18..0x20].copy_from_slice(&declared.to_be_bytes());
    let s = bind(&a);
    assert!(!s.layout.archive_size_is_file_len);
    assert_eq!(s.header.archive_size, declared);
    let size_item = s
        .layout
        .qualification_items
        .iter()
        .find(|i| i.subject == "header archive_size")
        .unwrap();
    assert_eq!(size_item.status, QualificationStatus::Unresolved);
    assert!(size_item.note.contains("preserved verbatim"));
    // Same-length replacement: header word untouched, output verified.
    let new = b"<meta/>NEW-BYTES".to_vec();
    let plan = plan(&s, vec![replacement(&s, 2, &new, false)]).unwrap();
    assert!(plan.header_edit.is_none());
    assert_eq!(plan.expected_archive_size, declared);
    let (out, _, v) = stage_verify(&a, &s, &plan);
    assert_eq!(out.len(), a.len());
    assert_eq!(&out[0x18..0x20], &declared.to_be_bytes());
    assert_eq!(read_entry(&out, s.entries[2].offset, 16), &new[..]);
    assert_eq!(v.entries_reparsed, 4);
    assert!(plan
        .qualification
        .items
        .iter()
        .any(|i| i.subject == "header archive_size" && i.status == QualificationStatus::Unresolved));
    // Growth (append-and-retarget) is refused on such an archive: the header word's meaning is
    // not established, so the stager never invents a new one.
    let bigger = noise(5000, 9);
    let err = super::plan(&s, vec![replacement(&s, 3, &bigger, false)]).unwrap_err();
    assert!(
        matches!(err, StageError::LengthChangeOnUnresolvedArchiveSize { .. }),
        "{err}"
    );
    // The Legacy layout keeps its strict rule.
    let mut legacy = Synth::new().build();
    let d = legacy.len() as u64 - 4096;
    legacy[0x18..0x20].copy_from_slice(&d.to_be_bytes());
    let err = BoundSource::bind(&mut Cursor::new(legacy)).unwrap_err();
    assert!(matches!(err, StageError::ArchiveSizeMismatch { .. }), "{err}");
}

/// The tolerance is exactly the measured shape: a declared size below the length is accepted only
/// when the entries end at the file length. Trailing bytes no entry covers, or a declared size
/// above the length, are refused before anything is read further.
#[test]
fn nhl15_declared_archive_size_is_refused_when_it_does_not_match_the_measured_shape() {
    let mut synth = Synth::new();
    synth.version = QUALIFIED_VERSION_PS4_16;
    let base = synth.build();
    // Declared below the length, but the file carries a trailing line no entry covers.
    let mut a = base.clone();
    let declared = a.len() as u64 - 16;
    a[0x18..0x20].copy_from_slice(&declared.to_be_bytes());
    a.extend_from_slice(&[0u8; 16]);
    let err = BoundSource::bind(&mut Cursor::new(a)).unwrap_err();
    assert!(
        matches!(err, StageError::ArchiveSizeInconsistent { .. }),
        "{err}"
    );
    // Declared above the length (a truncated file).
    let mut b = base.clone();
    let declared = base.len() as u64 + 16;
    b[0x18..0x20].copy_from_slice(&declared.to_be_bytes());
    let err = BoundSource::bind(&mut Cursor::new(b)).unwrap_err();
    assert!(matches!(err, StageError::ArchiveSizeMismatch { .. }), "{err}");
    // Truncated in the middle of the last entry: the span check refuses it.
    let mut c = base;
    c.truncate(c.len() - 16); // cuts the last entry (1 byte at the final line)
    let declared = c.len() as u64;
    c[0x18..0x20].copy_from_slice(&declared.to_be_bytes());
    let err = BoundSource::bind(&mut Cursor::new(c)).unwrap_err();
    assert!(matches!(err, StageError::SpanOutOfRange { .. }), "{err}");
}

/// D7 / research 02 §2 "private copy": a shared-span entry (the deduplicated PS4 logo archives)
/// is replaced by appending a private copy and retargeting **only** the chosen row — never in
/// place, even when the new bytes would fit — the sibling rows and the shared bytes stay, and the
/// siblings are named on the edit. The default rule still refuses.
#[test]
fn a_shared_span_entry_takes_a_private_copy_and_its_siblings_keep_the_block() {
    let a = Synth::new().build();
    // Entry 1 pointed at entry 0's block (and size): the two rows share one span.
    let mut c = a.clone();
    let e0 = TABLE_OFF;
    let e1 = TABLE_OFF + 16;
    let row0 = c[e0..e0 + 12].to_vec();
    c[e1..e1 + 12].copy_from_slice(&row0);
    let s = bind(&c);
    assert!(s.entries[0].shared_span && s.entries[1].shared_span);
    let shared = s.entries[0].clone();
    assert_eq!((s.entries[1].offset, s.entries[1].size), (shared.offset, shared.size));

    // A payload smaller than the shared block: it would fit in place, but must not be placed there.
    let plain = noise(40, 3);
    assert!(compress_chunkzip(&plain).len() as u64 <= shared.size as u64);
    let r = replacement(&s, 1, &plain, true);
    assert!(matches!(plan(&s, vec![r.clone()]).unwrap_err(), StageError::SharedSpan { index: 1, .. }));
    let p = plan_with(&s, vec![r], SharedSpanRule::PrivateCopy).unwrap();
    let e = &p.edits[0];
    let new_offset = match e.placement {
        Placement::AppendAndRetarget { new_offset } => new_offset,
        ref other => panic!("private copy must append, got {other:?}"),
    };
    assert_eq!(new_offset, c.len() as u64);
    assert_eq!(e.shared_with, vec![s.entries[0].path.clone()]);
    let row = e.row_edit.as_ref().unwrap();
    assert_eq!(row.row_offset as usize, e1);

    let (out, _report, v) = stage_verify(&c, &s, &p);
    assert_eq!(read_entry(&out, new_offset, e.new_size), &compress_chunkzip(&plain)[..]);
    // The shared block and the sibling row are untouched; every other entry too.
    assert_eq!(read_entry(&out, shared.offset, shared.size), read_entry(&c, shared.offset, shared.size));
    assert_eq!(&out[e0..e0 + 16], &c[e0..e0 + 16], "sibling row unchanged");
    for other in s.entries.iter().filter(|x| x.index != 1) {
        assert_eq!(read_entry(&out, other.offset, other.size), read_entry(&c, other.offset, other.size));
    }
    assert!(v.untouched_bytes_compared > 0);
    // Re-bound, the sibling still resolves to the old block and the chosen row to the copy; the
    // span is no longer shared.
    let s2 = bind(&out);
    assert_eq!((s2.entries[0].offset, s2.entries[0].size), (shared.offset, shared.size));
    assert_eq!((s2.entries[1].offset, s2.entries[1].size), (new_offset, e.new_size));
    assert!(!s2.entries[0].shared_span && !s2.entries[1].shared_span);
    // An unshared entry under the same rule is placed as usual (no siblings named).
    let p = plan_with(&s, vec![replacement(&s, 2, b"<meta/>ra", false)], SharedSpanRule::PrivateCopy).unwrap();
    assert!(matches!(p.edits[0].placement, Placement::FitInSlot { .. }));
    assert!(p.edits[0].shared_with.is_empty());
}

/// C7 (overnight 2026-09-20) / research 02 §4.1 "whole group replace": under
/// [`SharedSpanRule::WholeGroup`] (plan-wide or by [`plan_scoped`]'s index list) every row of a
/// deduplicated block follows the new bytes — in place when they fit the shared slot (each row's
/// size word patched), appended once with every row re-pointed when they do not; the siblings are
/// named; an irregular group (a sibling overlapping with another offset/size) refuses.
#[test]
fn a_shared_span_entry_replaces_its_whole_group_in_place_or_appended_once() {
    let a = Synth::new().build();
    let mut c = a.clone();
    let e0 = TABLE_OFF;
    let e1 = TABLE_OFF + 16;
    let row0 = c[e0..e0 + 12].to_vec();
    c[e1..e1 + 12].copy_from_slice(&row0);
    let s = bind(&c);
    let shared = s.entries[0].clone();
    // Fits the shared slot: in place, both size words patched, nothing appended.
    let plain = noise(40, 3);
    let stored = compress_chunkzip(&plain);
    assert!((stored.len() as u64) <= shared.slot_end - shared.offset);
    let p = plan_scoped(&s, vec![replacement(&s, 1, &plain, true)], SharedSpanRule::PrivateCopy, &[1]).unwrap();
    let e = &p.edits[0];
    assert!(matches!(e.placement, Placement::FitInSlot { .. }), "{:?}", e.placement);
    assert_eq!(e.shared_with, vec![s.entries[0].path.clone()]);
    assert_eq!(e.sibling_row_edits.len(), 1);
    assert_eq!(e.sibling_row_edits[0].row_offset as usize, e0);
    assert_eq!(p.new_eof, p.original_eof);
    let (out, _report, _v) = stage_verify(&c, &s, &p);
    let s2 = bind(&out);
    assert_eq!((s2.entries[0].offset, s2.entries[0].size), (shared.offset, stored.len() as u32));
    assert_eq!((s2.entries[1].offset, s2.entries[1].size), (shared.offset, stored.len() as u32));
    assert!(s2.entries[0].shared_span && s2.entries[1].shared_span, "still one block");
    assert_eq!(read_entry(&out, shared.offset, stored.len() as u32), &stored[..]);
    // Does not fit: appended once, both rows re-pointed at it, the old block left in place.
    let big = noise(4000, 5);
    let big_stored = compress_chunkzip(&big);
    assert!((big_stored.len() as u64) > shared.slot_end - shared.offset);
    let p = plan_with(&s, vec![replacement(&s, 0, &big, true)], SharedSpanRule::WholeGroup).unwrap();
    let e = &p.edits[0];
    let new_offset = match e.placement {
        Placement::AppendAndRetarget { new_offset } => new_offset,
        ref other => panic!("expected an append, got {other:?}"),
    };
    assert_eq!(e.sibling_row_edits.len(), 1);
    let (out, _report, _v) = stage_verify(&c, &s, &p);
    let s2 = bind(&out);
    assert_eq!((s2.entries[0].offset, s2.entries[0].size), (new_offset, big_stored.len() as u32));
    assert_eq!((s2.entries[1].offset, s2.entries[1].size), (new_offset, big_stored.len() as u32));
    assert_eq!(read_entry(&out, shared.offset, shared.size), read_entry(&c, shared.offset, shared.size), "old block left in place");
    // Irregular group: a sibling overlapping with a different size refuses.
    let mut d = c.clone();
    d[e1 + 8..e1 + 12].copy_from_slice(&(shared.size - 4).to_be_bytes());
    let s3 = bind(&d);
    assert!(s3.entries[0].shared_span);
    let err = plan_with(&s3, vec![replacement(&s3, 0, &plain, true)], SharedSpanRule::WholeGroup).unwrap_err();
    assert!(matches!(err, StageError::SharedGroupIrregular { .. }), "{err}");
    // The private-copy rule is untouched by the index list when the entry is not listed.
    let p = plan_scoped(&s, vec![replacement(&s, 1, &plain, true)], SharedSpanRule::PrivateCopy, &[0]).unwrap();
    assert!(matches!(p.edits[0].placement, Placement::AppendAndRetarget { .. }));
    assert!(p.edits[0].sibling_row_edits.is_empty());
}

/// R1 F1 (20 September 2026): two replacements that both own one file-table row — a whole-group
/// request owns every sibling row of its block — contradict each other and are refused at planning
/// with both paths and the contested row named, before any placement (the reviewer's reproduction
/// planned such a pair and then hit `patched_prefix`'s debug assertion at staging). Covers whole vs
/// whole (different and identical bytes, fit and append), whole vs private sibling in both orders,
/// and shows that two private copies, two separate groups and unshared entries still plan, stage,
/// verify and re-bind as before.
#[test]
fn contradictory_whole_group_and_private_scopes_on_one_block_are_refused_at_planning() {
    // Entries 0 and 1 alias one block (rows 2 and 3 stay separate).
    let a = Synth::new().build();
    let mut c = a.clone();
    let e0 = TABLE_OFF;
    let e1 = TABLE_OFF + 16;
    let row0 = c[e0..e0 + 12].to_vec();
    c[e1..e1 + 12].copy_from_slice(&row0);
    let s = bind(&c);
    assert!(s.entries[0].shared_span && s.entries[1].shared_span);
    let shared = s.entries[0].clone();
    let small = noise(40, 3); // fits the shared slot
    let small2 = noise(40, 5); // fits too, different bytes
    let big = noise(4000, 7); // appends
    assert!((compress_chunkzip(&small).len() as u64) <= shared.slot_end - shared.offset);
    assert!((compress_chunkzip(&big).len() as u64) > shared.slot_end - shared.offset);

    let refused = |reqs: Vec<Replacement>, rule: SharedSpanRule, group: &[usize]| {
        let err = plan_scoped(&s, reqs, rule, group).unwrap_err();
        match err {
            StageError::ConflictingOwnership { index, path, other_index, other_path, contested } => {
                assert_eq!((index, other_index), (0, 1), "{path} / {other_path}");
                assert_eq!(path, s.entries[0].path);
                assert_eq!(other_path, s.entries[1].path);
                assert!(contested == s.entries[0].path || contested == s.entries[1].path, "{contested}");
            }
            other => panic!("expected ConflictingOwnership, got {other}"),
        }
    };
    // Whole vs whole: different bytes, identical bytes; fit and append; by the index list and by
    // the plan-wide rule.
    refused(vec![replacement(&s, 0, &small, true), replacement(&s, 1, &small2, true)], SharedSpanRule::PrivateCopy, &[0, 1]);
    refused(vec![replacement(&s, 0, &small, true), replacement(&s, 1, &small, true)], SharedSpanRule::PrivateCopy, &[0, 1]);
    refused(vec![replacement(&s, 0, &big, true), replacement(&s, 1, &big, true)], SharedSpanRule::PrivateCopy, &[0, 1]);
    refused(vec![replacement(&s, 0, &small, true), replacement(&s, 1, &big, true)], SharedSpanRule::WholeGroup, &[]);
    // Whole vs private sibling, both orders (the request order does not matter: the ownership is
    // resolved over the sorted set), fit and append.
    refused(vec![replacement(&s, 0, &small, true), replacement(&s, 1, &small2, true)], SharedSpanRule::PrivateCopy, &[0]);
    refused(vec![replacement(&s, 1, &small2, true), replacement(&s, 0, &small, true)], SharedSpanRule::PrivateCopy, &[0]);
    refused(vec![replacement(&s, 0, &small, true), replacement(&s, 1, &small2, true)], SharedSpanRule::PrivateCopy, &[1]);
    refused(vec![replacement(&s, 1, &big, true), replacement(&s, 0, &big, true)], SharedSpanRule::PrivateCopy, &[1]);
    // A whole-group request on the block plus an unrelated entry still plans and stages.
    let p = plan_scoped(&s, vec![replacement(&s, 0, &small, true), replacement(&s, 2, b"<meta/>ra", false)], SharedSpanRule::PrivateCopy, &[0]).unwrap();
    assert_eq!(p.edits.len(), 2);
    assert_eq!(p.edits[0].sibling_row_edits.len(), 1);
    let (out, _r, v) = stage_verify(&c, &s, &p);
    assert!(v.untouched_bytes_compared > 0);
    let s2 = bind(&out);
    assert_eq!((s2.entries[1].offset, s2.entries[1].size), (shared.offset, compress_chunkzip(&small).len() as u32), "sibling followed the group");
    // Two private copies on the block: each row takes its own appended copy; the block and every
    // other row stay; re-bound, nothing is shared any more.
    let p = plan_with(&s, vec![replacement(&s, 0, &small, true), replacement(&s, 1, &small2, true)], SharedSpanRule::PrivateCopy).unwrap();
    assert!(p.edits.iter().all(|e| matches!(e.placement, Placement::AppendAndRetarget { .. }) && e.sibling_row_edits.is_empty()));
    let (out, _r, _v) = stage_verify(&c, &s, &p);
    assert_eq!(read_entry(&out, shared.offset, shared.size), read_entry(&c, shared.offset, shared.size), "shared block untouched");
    let s2 = bind(&out);
    assert!(!s2.entries[0].shared_span && !s2.entries[1].shared_span);
    assert_eq!(read_entry(&out, s2.entries[0].offset, s2.entries[0].size), &compress_chunkzip(&small)[..]);
    assert_eq!(read_entry(&out, s2.entries[1].offset, s2.entries[1].size), &compress_chunkzip(&small2)[..]);
    for other in s.entries.iter().filter(|x| x.index > 1) {
        assert_eq!(read_entry(&out, other.offset, other.size), read_entry(&c, other.offset, other.size));
    }

    // Two separate deduplicated blocks (0/1 and 2/3): a whole-group request on each is not a
    // conflict — both blocks are rewritten, each with its own sibling row, and the output verifies.
    let mut d = c.clone();
    let e2 = TABLE_OFF + 32;
    let e3 = TABLE_OFF + 48;
    let row2 = d[e2..e2 + 12].to_vec();
    d[e3..e3 + 12].copy_from_slice(&row2);
    let s3 = bind(&d);
    assert!(s3.entries.iter().all(|e| e.shared_span));
    let p = plan_scoped(&s3, vec![replacement(&s3, 0, &small, true), replacement(&s3, 2, b"<meta/>ra", false)], SharedSpanRule::PrivateCopy, &[0, 2]).unwrap();
    assert_eq!(p.edits.len(), 2);
    assert_eq!(p.edits[0].sibling_row_edits[0].row_offset as usize, e1);
    assert_eq!(p.edits[1].sibling_row_edits[0].row_offset as usize, e3);
    let (out, _r, v) = stage_verify(&d, &s3, &p);
    assert!(v.untouched_bytes_compared > 0);
    let s4 = bind(&out);
    assert_eq!((s4.entries[0].offset, s4.entries[0].size), (s4.entries[1].offset, s4.entries[1].size));
    assert_eq!((s4.entries[2].offset, s4.entries[2].size), (s4.entries[3].offset, s4.entries[3].size));
    assert_eq!(s4.entries[2].size as usize, b"<meta/>ra".len());
    // A private copy on one block beside a whole-group request on the other is likewise fine.
    let p = plan_scoped(&s3, vec![replacement(&s3, 1, &small, true), replacement(&s3, 2, b"<meta/>ra", false)], SharedSpanRule::PrivateCopy, &[2]).unwrap();
    assert!(matches!(p.edits[0].placement, Placement::AppendAndRetarget { .. }));
    assert!(p.edits[0].sibling_row_edits.is_empty());
    assert_eq!(p.edits[1].sibling_row_edits.len(), 1);
    stage_verify(&d, &s3, &p);
    // The duplicate-index refusal is unchanged and still comes first.
    assert!(matches!(
        plan_scoped(&s, vec![replacement(&s, 0, &small, true), replacement(&s, 0, &small2, true)], SharedSpanRule::PrivateCopy, &[0]).unwrap_err(),
        StageError::DuplicateReplacement { index: 0, .. }
    ));
}
