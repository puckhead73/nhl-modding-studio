//! Read-only raw table export for exact local donor/roster evidence.
use std::{error::Error, fs};
use tdb_core::TdbFile;
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 4 {
        return Err("usage: db_table_json database table output.json".into());
    }
    let bytes = fs::read(&a[1])?;
    let db = TdbFile::parse(&bytes)?;
    let t = db.table(&a[2]).ok_or("table missing")?;
    let mut rows = Vec::new();
    for i in 0..u32::from(t.current_records) {
        let mut r = vec![format!("\"_row\":{i}")];
        for f in &t.fields {
            let value = if f.kind == 0 {
                match db.read_inline_str(t, i, f) {
                    Some(s) => quote(&String::from_utf8_lossy(s)),
                    None => "null".into(),
                }
            } else {
                db.read_u64(t, i, f)
                    .map(|v| v.to_string())
                    .unwrap_or("null".into())
            };
            r.push(format!("{}:{}", quote(&f.id.raw()), value));
        }
        rows.push(format!("{{{}}}", r.join(",")));
    }
    fs::write(&a[3], format!("[{}]", rows.join(",\n")))?;
    println!("EXPORTED {} {} records", a[2], rows.len());
    Ok(())
}
