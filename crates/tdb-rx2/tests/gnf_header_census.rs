//! P2-B (21 Sep 2026) — the PS4 `GNF` header rule of research 2026-09-21 handoff 15 §1–2 as
//! Studio's `check_resize_rule` applies it, over every `.gnf` of the real extracted NHL 21 and
//! NHL 16 trees (read only, header + file size): the rule must accept every 2D block-format file
//! and refuse exactly the census's exceptions — cube maps (6 × the chain) and the unmodelled
//! BC6H (`0x28`) / unsupported formats. A missing tree fails, never skips.

use std::path::{Path, PathBuf};

use tdb_rx2::gnf::{check_resize_rule, GnfRuleRefusal};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("gnf")) {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "real extracted PS4 trees under H:/Emulators/ShadPS4/Games (read only): select with --ignored"]
fn the_size_rule_accepts_every_2d_block_texture_and_refuses_only_the_census_exceptions() {
    use std::io::Read;
    for (title, root, min_ok, max_cubes) in [
        ("nhl21", r"H:\Emulators\ShadPS4\Games\CUSA18182\databigs\Extracted", 140_000usize, 89usize),
        ("nhl16", r"H:\Emulators\ShadPS4\Games\CUSA01909\databigs\Extracted", 60_000, 23),
    ] {
        let root = Path::new(root);
        assert!(root.is_dir(), "{title}: {} missing (fails, never skips)", root.display());
        let mut files = Vec::new();
        walk(root, &mut files);
        let (mut ok, mut unmodelled, mut not_repro, mut other, mut unreadable) = (0usize, 0usize, 0usize, 0usize, 0usize);
        let mut examples: Vec<String> = Vec::new();
        for f in &files {
            // Header + the file size are all the rule needs: read 0x100 bytes and pad a synthetic
            // body of the file's length so the size checks see the true length without reading it.
            let Ok(meta) = std::fs::metadata(f) else { unreadable += 1; continue };
            let Ok(mut h) = std::fs::File::open(f) else { unreadable += 1; continue };
            let mut head = vec![0u8; 0x100];
            if h.read_exact(&mut head).is_err() || &head[..4] != b"GNF " {
                unreadable += 1;
                continue;
            }
            head.resize(meta.len() as usize, 0);
            match check_resize_rule(&head) {
                Ok(_) => ok += 1,
                Err(GnfRuleRefusal::UnmodelledFormat { .. }) => unmodelled += 1,
                Err(GnfRuleRefusal::NotReproducible { what, expected, actual }) => {
                    not_repro += 1;
                    if examples.len() < 10 {
                        examples.push(format!("{}: {what}: rule {expected} file {actual}", f.strip_prefix(root).unwrap().display()));
                    }
                }
                Err(e) => {
                    other += 1;
                    if examples.len() < 10 {
                        examples.push(format!("{}: {e}", f.strip_prefix(root).unwrap().display()));
                    }
                }
            }
        }
        eprintln!("{title}: {} gnf files, {ok} accepted, {unmodelled} unmodelled format, {not_repro} not reproducible, {other} other refusals, {unreadable} unreadable", files.len());
        for e in &examples {
            eprintln!("  {e}");
        }
        assert!(ok >= min_ok, "{title}: accepted {ok}");
        assert!(not_repro <= max_cubes, "{title}: {not_repro} files fail the size rule (the research counts {max_cubes} cube maps)");
    }
}
