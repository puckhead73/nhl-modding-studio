//! Read-only asset-name inventory using Studio's existing Frostbite readers.
//! Run with the private Cargo manifest documented in the research report.
use std::{env, fs::File, io::{Read, Seek, SeekFrom, Write}, collections::HashMap, path::Path};
use tdb_frostbite::{install, manifest};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let root = Path::new(args.get(1).ok_or("expected installation root")?);
    let report = install::inspect(root)?;
    let mut files = HashMap::new();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for layer in &report.layers {
        for sb in &layer.superbundles {
            if !root.join(&layer.name).join(format!("{sb}.toc")).is_file() { continue; }
            let toc = install::read_toc(root, &layer.name, sb)?;
            let mut ordered: Vec<_> = toc.bundles.iter().enumerate().collect();
            ordered.sort_by_key(|(_, b)| (b.spans[0].cas, b.spans[0].offset));
            for (ordinal, bundle) in ordered {
                let s = &bundle.spans[0];
                if s.size as usize > install::MAX_ASSET { return Err("oversized manifest".into()); }
                if let std::collections::hash_map::Entry::Vacant(e) = files.entry(s.cas) {
                    e.insert(File::open(install::cas_path(root, s.cas)?)?);
                }
                let f = files.get_mut(&s.cas).unwrap();
                f.seek(SeekFrom::Start(s.offset as u64))?;
                let mut bytes = vec![0; s.size as usize];
                f.read_exact(&mut bytes)?;
                let assets = manifest::parse(&bytes)?;
                if assets.len() + 1 != bundle.spans.len() { return Err("span count mismatch".into()); }
                for (index, asset) in assets.iter().enumerate() {
                    let lower = asset.name.to_lowercase();
                    // Keep locations and types, never copy proprietary asset payloads into tracked data.
                    if ["head", "portrait", "simengine", "nhl_game", "ones", "allstar", "playerlooks"].iter().any(|s| lower.contains(s)) {
                        writeln!(out, "{}", serde_json::json!({"layer":layer.name,"superbundle":sb,"bundle":bundle.name,"ordinal":ordinal,"asset_index":index,"asset":asset,"span":bundle.spans[index+1]}))?;
                    }
                }
            }
            out.flush()?;
        }
    }
    Ok(())
}
