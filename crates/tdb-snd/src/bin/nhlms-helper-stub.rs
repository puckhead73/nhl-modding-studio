//! Test stub for the helper runner (`tdb_snd::helper`): behaves like a decoder helper would in each
//! failure mode, selected by the first argument. Not a decoder and never shipped with Studio.
//!   echo    — copy stdin to stdout (after reading all of it), exit 0
//!   sleep   — read stdin, then sleep 30 s (the runner's timeout / cancellation stop it)
//!   flood   — write 64 MiB of zeros to stdout without reading stdin
//!   fail    — write a message to stderr and exit 3
//!   stderr  — write 1 MiB to stderr, then echo
//!   pidsleep <file> — write this process id to <file>, read stdin, then sleep 30 s
use std::io::{Read, Write};

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let mut input = Vec::new();
    match mode.as_str() {
        "flood" => {
            let chunk = vec![0u8; 1 << 20];
            let mut o = std::io::stdout().lock();
            for _ in 0..64 {
                if o.write_all(&chunk).is_err() {
                    return;
                }
            }
        }
        "sleep" => {
            let _ = std::io::stdin().read_to_end(&mut input);
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        "pidsleep" => {
            let file = std::env::args().nth(2).expect("pid file");
            std::fs::write(file, std::process::id().to_string()).expect("pid file written");
            let _ = std::io::stdin().read_to_end(&mut input);
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        "fail" => {
            eprintln!("stub: decoder refused the input");
            std::process::exit(3);
        }
        "stderr" => {
            let _ = std::io::stderr().write_all(&vec![b'e'; 1 << 20]);
            let _ = std::io::stdin().read_to_end(&mut input);
            let _ = std::io::stdout().write_all(&input);
        }
        _ => {
            let _ = std::io::stdin().read_to_end(&mut input);
            let _ = std::io::stdout().write_all(&input);
        }
    }
}
