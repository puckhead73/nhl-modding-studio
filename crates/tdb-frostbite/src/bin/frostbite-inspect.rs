//! Reproducible read-only corpus audit; stdout contains metadata, never payload bytes.
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
use tdb_frostbite::{install, manifest};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(
        args.get(1)
            .ok_or("usage: frostbite-inspect <install> [--audit|--names]")?,
    );
    let report = install::inspect(root)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    let names_only = args.iter().any(|a| a == "--names");
    if names_only || args.iter().any(|a| a == "--audit") {
        let mut files: HashMap<u32, (File, u64)> = HashMap::new();
        let mut base_names: HashMap<String, HashSet<String>> = HashMap::new();
        for layer in &report.layers {
            for sb in &layer.superbundles {
                if !root.join(&layer.name).join(format!("{sb}.toc")).is_file() {
                    println!(
                        "{}",
                        serde_json::json!({"layer":layer.name,"superbundle":sb,"status":"missing TOC; no overlay inferred"})
                    );
                    if layer.name == "Data" {
                        return Err("missing base TOC".into());
                    }
                    continue;
                }
                let toc = install::read_toc(root, &layer.name, sb)?;
                let names: HashSet<_> = toc.bundles.iter().map(|b| b.name.clone()).collect();
                if names.len() != toc.bundles.len() {
                    return Err(format!("duplicate bundle names in {}/{sb}", layer.name).into());
                }
                let matched = base_names
                    .get(sb)
                    .map(|base| names.intersection(base).count());
                if layer.name == "Data" {
                    base_names.insert(sb.clone(), names);
                }
                if names_only {
                    println!(
                        "{}",
                        serde_json::json!({"layer":layer.name,"superbundle":sb,"bundles":toc.bundles.len(),"base_name_matches":matched})
                    );
                    continue;
                }
                for c in &toc.chunks {
                    if let std::collections::hash_map::Entry::Vacant(e) = files.entry(c.span.cas) {
                        let f = File::open(install::cas_path(root, c.span.cas)?)?;
                        let len = f.metadata()?.len();
                        e.insert((f, len));
                    }
                    if c.span.offset as u64 + c.span.size as u64 > files[&c.span.cas].1 {
                        return Err("TOC chunk beyond CAS".into());
                    }
                }
                let mut assets = 0;
                let mut empty = 0;
                // Bundle order is a hash/name order, not disk order. Sort reads so the
                // corpus audit remains practical on mechanical game-library drives.
                let mut ordered: Vec<_> = toc.bundles.iter().collect();
                ordered.sort_by_key(|b| (b.spans[0].cas, b.spans[0].offset));
                for b in ordered {
                    for s in &b.spans {
                        if let std::collections::hash_map::Entry::Vacant(e) = files.entry(s.cas) {
                            let f = File::open(install::cas_path(root, s.cas)?)?;
                            let len = f.metadata()?.len();
                            e.insert((f, len));
                        }
                        if s.offset as u64 + s.size as u64 > files[&s.cas].1 {
                            return Err("span beyond CAS".into());
                        }
                    }
                    let s = &b.spans[0];
                    if s.size as usize > install::MAX_ASSET {
                        return Err("manifest too large".into());
                    }
                    let f = &mut files.get_mut(&s.cas).unwrap().0;
                    f.seek(SeekFrom::Start(s.offset as u64))?;
                    let mut bytes = vec![0; s.size as usize];
                    f.read_exact(&mut bytes)?;
                    let parsed = manifest::parse(&bytes)?;
                    if parsed.len() + 1 != b.spans.len() {
                        return Err(format!("{sb}/{} span count mismatch", b.name).into());
                    }
                    assets += parsed.len();
                    empty += usize::from(parsed.is_empty());
                }
                println!(
                    "{}",
                    serde_json::json!({"layer":layer.name,"superbundle":sb,"bundles":toc.bundles.len(),"assets":assets,"empty":empty,"toc_chunks":toc.chunks.len(),"first_bundle":toc.bundles.first().map(|b|&b.name)})
                );
                std::io::stdout().flush()?;
            }
        }
    }
    Ok(())
}
