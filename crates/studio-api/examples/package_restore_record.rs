//! Uninstall one mod-manager install through the installer's own restore: `RECORD_DIR` is the
//! install record folder (e.g. `%APPDATA%\com.nhlmoddingstudio.app\installs\7`). Prints the
//! outcome per destination. Nothing else is touched.
use studio_api::Registry;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: package_restore_record RECORD_DIR".into());
    }
    let out = Registry::default().package_restore(&args[1])?;
    println!("RESTORE seq {} state {} detail {:?}", out.seq, out.state, out.detail);
    for d in &out.destinations {
        println!("  {} ({}): {} -> {}", d.key, d.label, d.path, d.state);
    }
    if out.state != "restored" {
        return Err(format!("restore ended in state {}", out.state).into());
    }
    Ok(())
}
