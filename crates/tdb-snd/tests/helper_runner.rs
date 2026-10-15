//! The XMA2 helper runner's process discipline, against a stub helper (`nhlms-helper-stub`): echo
//! of a large input (concurrent stdin write and stdout drain — no pipe deadlock), a failing exit
//! with its stderr, a stderr flood beside a normal result, a stdout flood stopped at the cap, a
//! sleeping helper stopped by the timeout and by cancellation (the child is killed and reaped),
//! cancellation raised while the helper is running (the process is gone afterwards), a missing or
//! unusable helper file, and `locate` never falling back to `PATH`.
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use tdb_snd::helper::{locate, run, HelperError, Limits, XMA2_HELPER_ENV};

fn stub() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_nhlms-helper-stub"))
}

#[test]
fn echo_of_a_large_input_does_not_deadlock() {
    let input: Vec<u8> = (0..8 << 20).map(|i| (i % 251) as u8).collect();
    let out = run(stub(), &["echo"], &input, Limits::default(), None).unwrap();
    assert_eq!(out, input);
}

#[test]
fn failures_are_named_and_the_child_is_stopped() {
    let e = run(stub(), &["fail"], b"x", Limits::default(), None).unwrap_err();
    assert!(matches!(&e, HelperError::Failed { stderr, .. } if stderr.contains("decoder refused")), "{e}");
    // A stderr flood beyond its cap does not block a normal result.
    assert_eq!(run(stub(), &["stderr"], b"abc", Limits::default(), None).unwrap(), b"abc");
    let small = Limits { max_stdout: 1 << 20, ..Limits::default() };
    assert_eq!(run(stub(), &["flood"], b"", small, None).unwrap_err(), HelperError::OutputTooLarge(1 << 20));
    let t = Instant::now();
    let quick = Limits { timeout: Duration::from_millis(500), ..Limits::default() };
    assert_eq!(run(stub(), &["sleep"], b"", quick, None).unwrap_err(), HelperError::Timeout(Duration::from_millis(500)));
    assert!(t.elapsed() < Duration::from_secs(10), "stopped, not waited for: {:?}", t.elapsed());
    let cancel = AtomicBool::new(true);
    assert_eq!(run(stub(), &["sleep"], b"", Limits::default(), Some(&cancel)).unwrap_err(), HelperError::Cancelled);
    assert!(matches!(run(Path::new("Z:/no/such/helper.exe"), &["echo"], b"", Limits::default(), None), Err(HelperError::Spawn(_))));
}

#[test]
fn locate_uses_only_the_explicit_path_or_the_helpers_folder() {
    // Only this test touches the variable.
    std::env::remove_var(XMA2_HELPER_ENV);
    let dir = std::env::temp_dir().join(format!("nhlms-helper-locate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("helpers")).unwrap();
    let e = locate(Some(&dir)).unwrap_err().to_string();
    assert!(e.contains("no XMA2 decoder helper is installed") && e.contains(XMA2_HELPER_ENV), "{e}");
    let f = dir.join("helpers").join(tdb_snd::helper::XMA2_HELPER_FILE);
    std::fs::write(&f, b"").unwrap();
    assert_eq!(locate(Some(&dir)).unwrap(), f);
    std::env::set_var(XMA2_HELPER_ENV, dir.join("missing.exe"));
    assert!(locate(Some(&dir)).unwrap_err().to_string().contains("not a file"));
    std::env::remove_var(XMA2_HELPER_ENV);
    let _ = std::fs::remove_dir_all(&dir);
}

/// True while a process with this id is still listed by Windows (`tasklist`, a system tool run by
/// absolute path; used only by this test).
#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    let sys = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let out = std::process::Command::new(format!(r"{sys}\System32\tasklist.exe"))
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
        .expect("tasklist");
    String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\""))
}

#[cfg(windows)]
#[test]
fn cancellation_while_running_and_timeout_stop_and_reap_the_child() {
    assert!(process_alive(std::process::id()), "positive control: this test process is listed");
    let dir = std::env::temp_dir().join(format!("nhlms-helper-cancel-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (case, pid_file) in [("cancel", dir.join("cancel.pid")), ("timeout", dir.join("timeout.pid"))] {
        let arg = pid_file.to_string_lossy().into_owned();
        let cancel = AtomicBool::new(false);
        let t = Instant::now();
        let limits = if case == "timeout" { Limits { timeout: Duration::from_millis(800), ..Limits::default() } } else { Limits::default() };
        let result = std::thread::scope(|s| {
            if case == "cancel" {
                s.spawn(|| {
                    // Raise the flag only once the helper is demonstrably running.
                    while !pid_file.is_file() {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    std::thread::sleep(Duration::from_millis(200));
                    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
                });
            }
            run(stub(), &["pidsleep", &arg], b"", limits, Some(&cancel))
        });
        let elapsed = t.elapsed();
        let expected = if case == "cancel" { HelperError::Cancelled } else { HelperError::Timeout(Duration::from_millis(800)) };
        assert_eq!(result.unwrap_err(), expected, "{case}");
        assert!(elapsed < Duration::from_secs(10), "{case}: stopped, not waited for ({elapsed:?})");
        let pid: u32 = std::fs::read_to_string(&pid_file).unwrap().trim().parse().unwrap();
        assert!(!process_alive(pid), "{case}: helper process {pid} still running after the runner returned");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unusable_helper_file_refuses_by_name() {
    let dir = std::env::temp_dir().join(format!("nhlms-helper-unusable-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join(tdb_snd::helper::XMA2_HELPER_FILE);
    std::fs::write(&f, b"not an executable").unwrap();
    let e = run(&f, &["echo"], b"", Limits::default(), None).unwrap_err();
    assert!(matches!(&e, HelperError::Spawn(m) if m.contains(tdb_snd::helper::XMA2_HELPER_FILE)), "{e}");
    assert!(e.to_string().starts_with("the decoder helper could not start"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}
