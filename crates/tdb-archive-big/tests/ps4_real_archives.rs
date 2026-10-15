//! Real PS4 `EB\0\3` archives (20-byte rows) through the S2-A planner (O7 / L1-support-EB).
//!
//! Private fixtures: `NHL_STUDIO_PS4_BIGS` names a directory holding a title's `databigs`
//! (e.g. the CUSA18182 install's `databigs`). The source archives are opened read-only; staged
//! outputs go to `NHL_STUDIO_PS4_OUT` (required). A selected case with a missing fixture
//! **fails** (never a silent skip).
//!
//! ```sh
//! NHL_STUDIO_PS4_BIGS=<databigs> NHL_STUDIO_PS4_OUT=<scratch> \
//!   cargo test -p tdb-archive-big --test ps4_real_archives -- --ignored --nocapture
//! ```

use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;

use tdb_archive_big::stage::{
    plan, stage, verify, BoundSource, ExpectedDecoded, Placement, QualificationStatus, Replacement,
    ROW_LEN_PS4,
};

fn fixture_dir() -> PathBuf {
    let d = std::env::var("NHL_STUDIO_PS4_BIGS")
        .expect("NHL_STUDIO_PS4_BIGS must point at a databigs directory");
    let p = PathBuf::from(d);
    assert!(p.is_dir(), "{} is not a directory", p.display());
    p
}

fn out_dir() -> PathBuf {
    let d = std::env::var("NHL_STUDIO_PS4_OUT")
        .expect("NHL_STUDIO_PS4_OUT must name a scratch directory");
    let p = PathBuf::from(d);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn read_span(path: &PathBuf, off: u64, len: u32) -> Vec<u8> {
    let mut f = File::open(path).unwrap();
    f.seek(SeekFrom::Start(off)).unwrap();
    let mut v = vec![0u8; len as usize];
    f.read_exact(&mut v).unwrap();
    v
}

/// The row facts the 20-byte planner relies on hold on real archives: every `+16` word is the
/// path hash, `+4` is zero, `+12` is preserved as unresolved.
#[test]
#[ignore]
fn real_ps4_archives_bind_with_verified_path_hashes() {
    let dir = fixture_dir();
    for name in ["boot.big", "overlays_21.big", "inxp_00.big"] {
        let path = dir.join(name);
        assert!(path.is_file(), "missing fixture {}", path.display());
        let mut f = BufReader::new(File::open(&path).unwrap());
        let s = BoundSource::bind(&mut f).unwrap_or_else(|e| panic!("{name}: {e}"));
        println!(
            "{name}: {} entries, row_len {}, path_hash_verified {}, word_4_all_zero {}, padding_all_zero {}, blockers {}",
            s.entries.len(),
            s.layout.row_len,
            s.layout.path_hash_verified,
            s.layout.word_4_all_zero,
            s.layout.padding_all_zero,
            s.layout.deploy_blockers.len()
        );
        assert_eq!(s.layout.row_len, ROW_LEN_PS4);
        assert!(s.layout.path_hash_verified, "{name}");
        assert!(s.layout.word_4_all_zero, "{name}");
        assert!(
            s.layout.deploy_blockers.is_empty(),
            "{name}: {:?}",
            s.layout.deploy_blockers
        );
        let sort_key = s
            .layout
            .qualification_items
            .iter()
            .find(|i| i.subject == "row +12 word (PS4 sort key)")
            .unwrap();
        assert_eq!(sort_key.status, QualificationStatus::Unresolved);
        // Measured claim behind the sort-key note: rows are sorted by the +12 word.
        assert!(
            s.entries.windows(2).all(|w| w[0].word_12 <= w[1].word_12),
            "{name}: +12 sorted"
        );
    }
}

/// Replace the NHL 21 main database member inside a staged copy of `inxp_00.big`: no-op
/// reproduces the source; a same-length replacement fits in place; every other member keeps
/// its bytes and its row; the output re-binds and verifies. The source is never written.
#[test]
#[ignore]
fn inxp00_nhl_game_db_member_replaces_in_place() {
    let dir = fixture_dir();
    let src_path = dir.join("inxp_00.big");
    assert!(src_path.is_file(), "missing fixture {}", src_path.display());
    let out_dir = out_dir();
    let mut src = BufReader::new(File::open(&src_path).unwrap());
    let s = BoundSource::bind(&mut src).unwrap();
    let (idx, entry) = s
        .entries
        .iter()
        .enumerate()
        .find(|(_, e)| e.path.ends_with("/nhl_game.db"))
        .expect("inxp_00.big carries nhl_game.db");
    println!(
        "nhl_game.db member: entry {idx} `{}` offset {} size {} form {:?} shared {}",
        entry.path, entry.offset, entry.size, entry.stored_form, entry.shared_span
    );
    assert!(!entry.shared_span);
    let stored = read_span(&src_path, entry.offset, entry.size);
    let expected = ExpectedDecoded::of(&stored);

    // No-op plan: staging reproduces the source hash exactly.
    let p = plan(
        &s,
        vec![Replacement {
            entry_index: idx,
            expected_path: entry.path.clone(),
            expected_stored_sha256: entry.stored_sha256,
            new_stored: stored.clone(),
            expected_decoded: expected.clone(),
        }],
    )
    .unwrap();
    assert!(p.is_noop());

    // Same-length replacement: flip one byte deep inside the raw member (a database edit keeps
    // the file length; this proves the placement, not a valid database).
    let mut edited = stored.clone();
    let i = edited.len() / 2;
    edited[i] ^= 0x01;
    let p = plan(
        &s,
        vec![Replacement {
            entry_index: idx,
            expected_path: entry.path.clone(),
            expected_stored_sha256: entry.stored_sha256,
            new_stored: edited.clone(),
            expected_decoded: ExpectedDecoded::of(&edited),
        }],
    )
    .unwrap();
    assert!(matches!(p.edits[0].placement, Placement::FitInSlot { .. }));
    assert!(
        p.edits[0].row_edit.is_none(),
        "same length: the row is unchanged"
    );

    let out_path = out_dir.join("inxp_00.staged.big");
    {
        let mut src = BufReader::new(File::open(&src_path).unwrap());
        let mut out = BufWriter::new(File::create(&out_path).unwrap());
        let report = stage(&s, &p, &mut src, &mut out).unwrap();
        assert_eq!(report.original_eof, report.new_eof);
        assert!(
            !report.deployable(),
            "20-byte rows are never deployable-qualified here"
        );
        let mut out_r = BufReader::new(File::open(&out_path).unwrap());
        let mut src_r = BufReader::new(File::open(&src_path).unwrap());
        let v = verify(&s, &p, &report, &mut out_r, &mut src_r).unwrap();
        println!("verify: {v:?}");
    }
    // Re-bind the output: every other member's digest and row are unchanged.
    let mut out_r = BufReader::new(File::open(&out_path).unwrap());
    let re = BoundSource::bind(&mut out_r).unwrap();
    assert_eq!(re.entries.len(), s.entries.len());
    for (a, b) in s.entries.iter().zip(re.entries.iter()) {
        assert_eq!(a.path, b.path);
        assert_eq!(a.offset, b.offset);
        assert_eq!(a.size, b.size);
        assert_eq!(a.word_12, b.word_12);
        assert_eq!(a.word_16, b.word_16);
        if a.index != idx {
            assert_eq!(a.stored_sha256, b.stored_sha256, "{}", a.path);
        }
    }
    assert_eq!(read_span(&out_path, entry.offset, entry.size), edited);
    // A stale binding is refused: the same plan against the staged output as source.
    let mut cur = Cursor::new(Vec::new());
    assert!(stage(
        &s,
        &p,
        &mut BufReader::new(File::open(&out_path).unwrap()),
        &mut cur
    )
    .is_err());
    std::fs::remove_file(&out_path).unwrap();
}

// ------------------------------------------------------------- EB-V1: NHL 15 PS4 (0x0010_0400)

/// Every one of the 17 CUSA00561 archives binds: 16-byte rows, path hash verified on every row,
/// `+4` zero, and exactly the three archives characterised in EB-V1 carry a declared
/// `archive_size` below the file length (recorded as unresolved, never "fixed").
///
/// ```sh
/// NHL_STUDIO_PS4_BIGS=<CUSA00561/databigs> NHL_STUDIO_PS4_OUT=<scratch> \
///   cargo test -p tdb-archive-big --test ps4_real_archives nhl15 -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn nhl15_all_seventeen_archives_bind_with_the_declared_size_discrepancy_recorded() {
    let dir = fixture_dir();
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".big"))
        .collect();
    names.sort();
    assert_eq!(names.len(), 17, "expected the 17 NHL 15 archives, found {names:?}");
    let mut short: Vec<String> = Vec::new();
    for name in &names {
        let path = dir.join(name);
        let mut f = BufReader::new(File::open(&path).unwrap());
        let s = BoundSource::bind(&mut f).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(s.layout.version, 0x0010_0400, "{name}");
        assert_eq!(s.layout.row_len, 16, "{name}");
        assert!(s.layout.path_hash_verified, "{name}");
        assert!(s.layout.word_4_all_zero, "{name}");
        assert!(s.layout.deploy_blockers.is_empty(), "{name}: {:?}", s.layout.deploy_blockers);
        let size_item = s
            .layout
            .qualification_items
            .iter()
            .find(|i| i.subject == "header archive_size")
            .unwrap();
        println!(
            "{name}: {} entries, declared {} len {} ({:?}), padding_all_zero {}",
            s.entries.len(),
            s.header.archive_size,
            s.identity.len,
            size_item.status,
            s.layout.padding_all_zero
        );
        if !s.layout.archive_size_is_file_len {
            assert_eq!(size_item.status, QualificationStatus::Unresolved);
            short.push(name.clone());
        } else {
            assert_eq!(size_item.status, QualificationStatus::Resolved);
        }
    }
    assert_eq!(short, ["game20.big", "inxp20.big", "patch_inxp00.big"]);
}

/// The database member replaces in place on both the base archive (`inxp00.big`, declared size =
/// length) and the patch archive (`patch_inxp00.big`, declared size < length): the header word is
/// preserved verbatim, every other member and row is unchanged, and a longer member is refused on
/// the patch archive.
#[test]
#[ignore]
fn nhl15_nhl_game_db_member_replaces_in_place_on_base_and_patch_archives() {
    let dir = fixture_dir();
    let out_dir = out_dir();
    for (name, declared_is_len) in [("inxp00.big", true), ("patch_inxp00.big", false)] {
        let src_path = dir.join(name);
        let mut src = BufReader::new(File::open(&src_path).unwrap());
        let s = BoundSource::bind(&mut src).unwrap();
        assert_eq!(s.layout.archive_size_is_file_len, declared_is_len, "{name}");
        let (idx, entry) = s
            .entries
            .iter()
            .enumerate()
            .find(|(_, e)| e.path == "nhl/data/common/gamemodes/database/t3db/nhl_game.db")
            .expect("nhl_game.db member");
        assert!(!entry.shared_span);
        let stored = read_span(&src_path, entry.offset, entry.size);
        let mut edited = stored.clone();
        let i = edited.len() / 2;
        edited[i] ^= 0x01;
        let p = plan(
            &s,
            vec![Replacement {
                entry_index: idx,
                expected_path: entry.path.clone(),
                expected_stored_sha256: entry.stored_sha256,
                new_stored: edited.clone(),
                expected_decoded: ExpectedDecoded::of(&edited),
            }],
        )
        .unwrap();
        assert!(matches!(p.edits[0].placement, Placement::FitInSlot { .. }));
        assert!(p.header_edit.is_none());
        assert_eq!(p.expected_archive_size, s.header.archive_size);
        let out_path = out_dir.join(format!("{name}.staged"));
        {
            let mut src = BufReader::new(File::open(&src_path).unwrap());
            let mut out = BufWriter::new(File::create(&out_path).unwrap());
            let report = stage(&s, &p, &mut src, &mut out).unwrap();
            assert_eq!(report.original_eof, report.new_eof);
            assert!(!report.deployable());
            let mut out_r = BufReader::new(File::open(&out_path).unwrap());
            let mut src_r = BufReader::new(File::open(&src_path).unwrap());
            verify(&s, &p, &report, &mut out_r, &mut src_r).unwrap();
        }
        let mut out_r = BufReader::new(File::open(&out_path).unwrap());
        let re = BoundSource::bind(&mut out_r).unwrap();
        assert_eq!(re.header.archive_size, s.header.archive_size, "{name}: header word preserved");
        assert_eq!(re.entries.len(), s.entries.len());
        for (a, b) in s.entries.iter().zip(re.entries.iter()) {
            assert_eq!((a.path.as_str(), a.offset, a.size, a.word_12), (b.path.as_str(), b.offset, b.size, b.word_12));
            if a.index != idx {
                assert_eq!(a.stored_sha256, b.stored_sha256, "{name}: {}", a.path);
            }
        }
        assert_eq!(read_span(&out_path, entry.offset, entry.size), edited);
        println!("{name}: member {idx} replaced in place; declared archive_size {} preserved", s.header.archive_size);
        // A longer member cannot be placed on the patch archive (declared size unresolved).
        if !declared_is_len {
            let mut longer = stored.clone();
            longer.extend_from_slice(&[0u8; 64]);
            let err = plan(
                &s,
                vec![Replacement {
                    entry_index: idx,
                    expected_path: entry.path.clone(),
                    expected_stored_sha256: entry.stored_sha256,
                    new_stored: longer.clone(),
                    expected_decoded: ExpectedDecoded::of(&longer),
                }],
            )
            .unwrap_err();
            println!("{name}: growth refused: {err}");
        }
        std::fs::remove_file(&out_path).unwrap();
    }
}
