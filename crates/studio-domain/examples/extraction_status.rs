//! Read-only extraction diagnostics for an existing installation.
fn main() {
    let root = std::env::args().nth(1).expect("usage: extraction_status <game folder>");
    let pre = studio_domain::extract_service::preflight(std::path::Path::new(&root), studio_domain::Platform::Xbox);
    println!("{}: {}", pre.state, pre.reason);
    for archive in pre.archives {
        println!("{}: {} {:?}", archive.path.display(), archive.state, archive.note);
        if archive.state != "complete" {
            println!("members: {}", archive.members.len());
            for member in archive.members.iter().filter(|m| !matches!(m.state, "present" | "vanilla")) {
                println!("  {}: {}", member.name, member.state);
            }
        }
    }
}
