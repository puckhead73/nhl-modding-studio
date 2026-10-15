//! `mcd_probe` — read-only structural report for one PS4 `.mcd` (RSF) container.
//!
//! ```text
//! cargo run -p tdb-rx2 --example mcd_probe -- <file.mcd>
//! ```
//!
//! Prints the container size, every chunk marker's offset, and the submeshes the
//! production [`mcd::parse_player_mesh`] decoder recovers. Purely diagnostic: it
//! writes nothing and asserts no ownership graph beyond what the decoder claims.

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: mcd_probe <file.mcd>");
        std::process::exit(2);
    };
    let data = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("read {path}: {e}");
        std::process::exit(1);
    });
    println!("# {path}: {} bytes", data.len());

    for (tag, label) in [
        (b"TIEW", "VIEW"),
        (b"MOEG", "GEOM"),
        (b"TMFV", "VFMT"),
        (b"MRTS", "STRM"),
        (b"XDNI", "INDX"),
        (b"HSEM", "MESH"),
    ] {
        let hits: Vec<usize> = data
            .windows(4)
            .enumerate()
            .filter(|(_, w)| *w == tag)
            .map(|(i, _)| i)
            .collect();
        let head: Vec<String> = hits.iter().take(8).map(|o| format!("{o}")).collect();
        println!("{label}\tcount={}\tfirst=[{}]", hits.len(), head.join(","));
    }

    match tdb_rx2::mcd::parse_player_mesh(&data) {
        Ok(asset) => {
            println!("# parsed {} submesh(es)", asset.submeshes.len());
            for (i, s) in asset.submeshes.iter().enumerate() {
                println!(
                    "sub{i}\tname={:?}\tverts={}\ttris={}",
                    s.name,
                    s.vertices.len(),
                    s.triangles.len()
                );
            }
        }
        Err(e) => println!("# parse FAILED: {e}"),
    }
}
