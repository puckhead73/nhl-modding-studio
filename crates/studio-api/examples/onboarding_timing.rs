//! Read-only source discovery/extraction timing; never writes into the game tree.
use std::{path::Path, time::Instant};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let platform = if args[1] == "ps3" { studio_domain::Platform::Ps3 } else { studio_domain::Platform::Xbox };
    let root = Path::new(&args[2]);
    let start = Instant::now();
    let report = studio_domain::detect_install("nhllegacy", platform, root, None);
    println!("discovery {:?}: {:?}", start.elapsed(), report.install_dir);
    let start = Instant::now();
    let pre = studio_domain::extract_service::preflight(Path::new(report.install_dir.as_deref().unwrap_or(&args[2])), platform);
    println!("preflight {:?}: {} archives, {} members, {}", start.elapsed(), pre.archives.len(), pre.members_total, pre.state);
}
