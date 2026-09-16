//! `as7_portrait_census` — package C0-AS7 (D4): census every file of the two portrait folders of
//! one installed copy, per id, and write the record `tdb_canonical::asset_census` joins to the
//! anchor.
//!
//! Read-only. Walks `<root>/fe/ion/artassets/{playerheads,playerheadssmall}/<bucket>/<file>` in
//! sorted order and writes:
//!
//! * `<out>/<label>.csv` — one row per file: folder, bucket, file name, label (`current` /
//!   `backup-labelled` / `other`), the numeric id the name spells, size, SHA-256, and a shallow
//!   look at the `BIGF` tables (member count, member 3's name and stored length, the RefPack
//!   header's declared length). No payload is decompressed.
//! * `<out>/<label>.json` — specimen attribution, tool provenance, counts and the CSV's digest.
//!
//! ```sh
//! cargo run -p tdb-canonical --example as7_portrait_census -- \
//!     --root <USRDIR> --label <record-label> --out <dir> \
//!     --specimen <corpus-id> --target <target-id> \
//!     --identity "<how the copy was identified>" --source-commit <sha>
//! ```

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tdb_canonical::asset_census::{
    classify_portrait_name, peek_bigf_portrait, render_portrait_census_csv, PortraitFile,
    PortraitLabel, PORTRAIT_CENSUS_SCHEMA,
};

const FOLDERS: [&str; 2] = ["playerheads", "playerheadssmall"];
const SELF_SOURCE: &[u8] = include_bytes!("as7_portrait_census.rs");
const MODULE_SOURCE: &[u8] = include_bytes!("../src/asset_census.rs");

struct Args {
    root: PathBuf,
    label: String,
    out: PathBuf,
    specimen: String,
    target: String,
    identity: String,
    source_commit: String,
}

fn parse_args() -> Args {
    let mut m: BTreeMap<String, String> = BTreeMap::new();
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let key = k
            .strip_prefix("--")
            .unwrap_or_else(|| panic!("unexpected argument {k}"))
            .to_string();
        let v = it.next().unwrap_or_else(|| panic!("--{key} needs a value"));
        m.insert(key, v);
    }
    let take = |k: &str| m.get(k).cloned().unwrap_or_else(|| panic!("missing --{k}"));
    Args {
        root: PathBuf::from(take("root")),
        label: take("label"),
        out: PathBuf::from(take("out")),
        specimen: take("specimen"),
        target: take("target"),
        identity: take("identity"),
        source_commit: take("source-commit"),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn jesc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn sorted_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn main() {
    let a = parse_args();
    let mut files: Vec<PortraitFile> = Vec::new();
    let mut per_folder: BTreeMap<&str, BTreeMap<&'static str, usize>> = BTreeMap::new();
    let mut buckets: BTreeMap<String, usize> = BTreeMap::new();
    for folder in FOLDERS {
        let fdir = a.root.join("fe/ion/artassets").join(folder);
        for bucket in sorted_names(&fdir) {
            let bdir = fdir.join(&bucket);
            if !bdir.is_dir() {
                panic!("{}: not a directory", bdir.display());
            }
            for name in sorted_names(&bdir) {
                let path = bdir.join(&name);
                let bytes =
                    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                let (label, numeric_id) = classify_portrait_name(&name);
                *per_folder
                    .entry(folder)
                    .or_default()
                    .entry(label.as_str())
                    .or_default() += 1;
                *buckets.entry(format!("{folder}/{bucket}")).or_default() += 1;
                files.push(PortraitFile {
                    folder: folder.to_string(),
                    bucket: bucket.clone(),
                    file_name: name.clone(),
                    label,
                    numeric_id,
                    size: bytes.len() as u64,
                    sha256: hex(&Sha256::digest(&bytes)),
                    peek: peek_bigf_portrait(&bytes),
                });
            }
        }
    }
    let csv = render_portrait_census_csv(&files);
    let not_bigf = files.iter().filter(|f| f.peek.members.is_none()).count();
    let not_six = files
        .iter()
        .filter(|f| f.peek.members.is_some_and(|m| m != 6))
        .count();
    let no_declared = files
        .iter()
        .filter(|f| f.peek.members.is_some() && f.peek.texture_declared_size.is_none())
        .count();
    let other = files
        .iter()
        .filter(|f| f.label == PortraitLabel::Other)
        .count();

    let mut j = String::new();
    j.push_str("{\n");
    j.push_str(&format!("  \"record\": \"{PORTRAIT_CENSUS_SCHEMA}\",\n"));
    j.push_str("  \"package\": \"C0-AS7\",\n");
    j.push_str(&format!("  \"label\": \"{}\",\n", jesc(&a.label)));
    j.push_str("  \"specimen\": {\n");
    j.push_str(&format!("    \"corpus\": \"{}\",\n", jesc(&a.specimen)));
    j.push_str(&format!("    \"target\": \"{}\",\n", jesc(&a.target)));
    j.push_str("    \"relative_root\": \"fe/ion/artassets\",\n");
    j.push_str(&format!("    \"identity\": \"{}\"\n", jesc(&a.identity)));
    j.push_str("  },\n");
    j.push_str("  \"tool\": {\n");
    j.push_str("    \"crate\": \"tdb-canonical\",\n");
    j.push_str("    \"module\": \"crates/tdb-canonical/src/asset_census.rs\",\n");
    j.push_str(&format!(
        "    \"module_sha256\": \"{}\",\n",
        hex(&Sha256::digest(MODULE_SOURCE))
    ));
    j.push_str("    \"example\": \"crates/tdb-canonical/examples/as7_portrait_census.rs\",\n");
    j.push_str(&format!(
        "    \"example_sha256\": \"{}\",\n",
        hex(&Sha256::digest(SELF_SOURCE))
    ));
    j.push_str(&format!(
        "    \"source_commit\": \"{}\"\n",
        jesc(&a.source_commit)
    ));
    j.push_str("  },\n");
    j.push_str("  \"counts\": {\n");
    j.push_str(&format!("    \"files\": {},\n", files.len()));
    j.push_str("    \"by_folder_and_label\": {\n");
    let nf = per_folder.len();
    for (k, (folder, labels)) in per_folder.iter().enumerate() {
        let inner: Vec<String> = labels
            .iter()
            .map(|(l, c)| format!("\"{l}\": {c}"))
            .collect();
        j.push_str(&format!(
            "      \"{folder}\": {{ {} }}{}\n",
            inner.join(", "),
            if k + 1 < nf { "," } else { "" }
        ));
    }
    j.push_str("    },\n");
    j.push_str("    \"by_bucket\": {\n");
    let nb = buckets.len();
    for (k, (b, c)) in buckets.iter().enumerate() {
        j.push_str(&format!(
            "      \"{b}\": {c}{}\n",
            if k + 1 < nb { "," } else { "" }
        ));
    }
    j.push_str("    },\n");
    j.push_str(&format!("    \"label_other\": {other},\n"));
    j.push_str(&format!("    \"not_bigf\": {not_bigf},\n"));
    j.push_str(&format!("    \"bigf_member_count_not_six\": {not_six},\n"));
    j.push_str(&format!(
        "    \"bigf_without_refpack_declared_size\": {no_declared}\n"
    ));
    j.push_str("  },\n");
    j.push_str(&format!("  \"census_csv\": \"{}.csv\",\n", jesc(&a.label)));
    j.push_str(&format!(
        "  \"census_csv_sha256\": \"{}\",\n",
        hex(&Sha256::digest(csv.as_bytes()))
    ));
    j.push_str("  \"limitations\": [\n");
    j.push_str("    \"Every file under the two folders was opened read-only and hashed; the numeric id is read from the file name only. Nothing binds a name to a database field here — the join to the anchor is a separate, re-derivable record.\",\n");
    j.push_str("    \"`label` is the file name's: `current` is `p<digits>.big`, `backup-labelled` is `p<digits>.big.bak`. Neither says who wrote the bytes or when.\",\n");
    j.push_str("    \"The `bigf_*` and `texture_*` cells come from the archive's own member table and the RefPack header of member 3 (the texture position C0-POR1 measured); nothing was decompressed, so the declared size is what the header says, not a measured payload.\",\n");
    j.push_str("    \"This copy was recorded by C0-POR1-R1 as an observed installed copy that is not established to be pristine or complete; `patch.big` at its root is measured separately (D3). The census describes this copy, not the target.\"\n");
    j.push_str("  ]\n}\n");

    std::fs::create_dir_all(&a.out).unwrap();
    let csv_path = a.out.join(format!("{}.csv", a.label));
    let json_path = a.out.join(format!("{}.json", a.label));
    std::fs::File::create(&csv_path)
        .unwrap()
        .write_all(csv.as_bytes())
        .unwrap();
    std::fs::File::create(&json_path)
        .unwrap()
        .write_all(j.as_bytes())
        .unwrap();
    println!(
        "{}: {} files ({other} other, {not_bigf} not BIGF)\n  -> {}\n  -> {}",
        a.label,
        files.len(),
        csv_path.display(),
        json_path.display()
    );
}
