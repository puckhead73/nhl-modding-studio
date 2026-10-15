//! Generate a reference from an original ISO, never from an already-extracted installation.
use std::{error::Error, fs::File, path::Path};
use studio_domain::game_manifest::{hash, Anchor, Entry, Manifest};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 8 {
        return Err("Usage: game-file-manifest <iso> <game-id> <platform> <edition-label> <anchor-dir> <output.json> <region>".into());
    }
    let before = std::fs::metadata(&args[1])?;
    let iso = tdb_xiso::Xiso::open(Path::new(&args[1]))?;
    let mut files = vec![];
    for m in iso.members().iter().filter(|m| !m.is_dir()) {
        eprintln!("Hashing {}", m.path);
        files.push(Entry {
            path: m.path.clone(),
            size: m.size as u64,
            sha256: hash(iso.member_reader(m)?)?,
            kind: if m.path.to_lowercase().ends_with(".big") {
                "container"
            } else {
                "loose"
            }
            .into(),
            optional: m.path.contains("$SystemUpdate"),
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    fn anchors(
        base: &Path,
        dir: &Path,
        files: &[Entry],
        out: &mut Vec<Anchor>,
    ) -> Result<(), Box<dyn Error>> {
        for e in std::fs::read_dir(dir)? {
            let p = e?.path();
            if p.is_dir() {
                anchors(base, &p, files, out)?;
            } else {
                let sha256 = hash(File::open(&p)?)?;
                out.push(Anchor {
                    path: p.strip_prefix(base)?.to_string_lossy().replace('\\', "/"),
                    disc_matches: files
                        .iter()
                        .filter(|f| f.sha256 == sha256)
                        .map(|f| f.path.clone())
                        .collect(),
                    sha256,
                });
            }
        }
        Ok(())
    }
    let anchor_dir = Path::new(&args[5]);
    let mut matched = vec![];
    for sub in ["databases", "savefiles"] {
        if anchor_dir.join(sub).is_dir() {
            anchors(anchor_dir, &anchor_dir.join(sub), &files, &mut matched)?;
        }
    }
    matched.sort_by(|a, b| a.path.cmp(&b.path));
    eprintln!("Hashing source ISO identity");
    let manifest = Manifest {
        schema: "nhlms-game-files/1".into(),
        game_id: args[2].clone(),
        platform: args[3].clone(),
        source: args[4].clone(),
        region: args[7].clone(),
        image_sha256: hash(File::open(&args[1])?)?,
        files,
        anchors: matched,
    };
    let after = std::fs::metadata(&args[1])?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err("Source ISO changed during manifest generation".into());
    }
    // Do not silently replace an existing approved reference.
    use std::io::Write;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[6])?;
    output.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    Ok(())
}
