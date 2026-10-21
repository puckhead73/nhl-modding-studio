//! Decode every indexed DDS through the actual Studio preview decoder.
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: head_library_texture_audit LIBRARY NEW_REPORT.json".into());
    }
    let root = std::fs::canonicalize(&args[1])?;
    let report = Path::new(&args[2]);
    if report.exists() {
        return Err("Choose a new report path".into());
    }
    let db = Connection::open_with_flags(
        root.join("headmeshes.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let mut query = db.prepare("SELECT textures_json FROM appearances")?;
    let mut paths = std::collections::BTreeSet::new();
    for row in query.query_map([], |r| r.get::<_, String>(0))? {
        let textures: Value = serde_json::from_str(&row?)?;
        for texture in textures.as_array().ok_or("Invalid texture array")? {
            paths.insert(
                texture["file"]
                    .as_str()
                    .ok_or("Missing texture path")?
                    .to_owned(),
            );
        }
    }
    let failures = std::sync::Mutex::new(Vec::new());
    let checked = std::sync::atomic::AtomicUsize::new(0);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let paths: Vec<_> = paths.into_iter().collect();
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(8);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(relative) = paths.get(index) else {
                    break;
                };
                let result = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let path = std::fs::canonicalize(root.join(relative))?;
                    if !path.starts_with(&root) {
                        return Err("Path escapes library".into());
                    }
                    let (w, h, pixels) =
                        studio_domain::head_library::texture_rgba(&std::fs::read(path)?)?;
                    if pixels.len() != w as usize * h as usize * 4 {
                        return Err("Decoded pixel size mismatch".into());
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    failures
                        .lock()
                        .unwrap()
                        .push(json!({"file":relative,"error":error.to_string()}));
                }
                let count = checked.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if count % 1000 == 0 {
                    println!(
                        "Decoded {count}/{} textures; {} errors",
                        paths.len(),
                        failures.lock().unwrap().len()
                    );
                }
            });
        }
    });
    let failures = failures.into_inner().unwrap();
    let checked = checked.into_inner();
    std::fs::write(
        report,
        serde_json::to_vec_pretty(&json!({
            "checked":checked, "passed":failures.is_empty(), "errors":failures,
            "scope":"Full DDS decoding through Studio; does not validate material assignments or native shaders."
        }))?,
    )?;
    if !failures.is_empty() {
        return Err(format!("{} textures failed decoding", failures.len()).into());
    }
    println!("All {checked} textures decoded.");
    Ok(())
}
