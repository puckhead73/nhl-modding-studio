//! Job execution — spawn, capture, timeout, collect response.
//!
//! One job = one working directory containing `request.json` (input), the materialized
//! Python script, `blender.log` (captured stdout+stderr), and `response.json` (the
//! script's last act). Any deviation — nonzero exit, timeout, missing or unparseable
//! response — resolves to a typed error carrying the log tail, so the failure the user
//! sees always includes what Blender actually said.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Default per-job wall-clock budget (a single-head wrap runs in seconds; ten minutes
/// means something is wedged).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);
/// How much of `blender.log` rides along on failures.
const LOG_TAIL_BYTES: usize = 4096;

/// The donor-wrap request (`op = "wrap_head"`). All tunables travel in the request —
/// the script has no built-in donor knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrapHeadRequest {
    pub op: String,
    /// Donor head geometry (OBJ, donor vertex order is the output order).
    pub donor_obj: String,
    /// Target head-only geometry (OBJ, positions-only is fine).
    pub target_obj: String,
    /// Output `.pv` positions file (one `x y z 0 0` line per donor vertex).
    pub out_pv: String,
    /// Optional front-view verification render.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_png: Option<String>,
    pub params: WrapHeadParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrapHeadParams {
    pub eye_center_l: [f32; 3],
    pub eye_center_r: [f32; 3],
    pub eye_freeze_inner_m: f32,
    pub eye_freeze_outer_m: f32,
    pub boundary_freeze_rings: u32,
    pub smooth_factor: f32,
    pub smooth_iterations: u32,
    pub eye_hole_y_range: [f32; 2],
    /// Catmull-Clark levels applied to sparse targets before wrapping (0 for dense
    /// scan/NHL24 meshes).
    pub subdivision_levels: u32,
    /// Expected donor vertex count; the script fails the job if the output would differ.
    pub expected_vertex_count: usize,
}

/// What the script reports back on success.
#[derive(Debug, Clone, Deserialize)]
pub struct WrapHeadResponse {
    pub status: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub vertex_count: usize,
    /// Vertices fully frozen by the eye-pocket rule (must be > 0 on a real head —
    /// a zero here means the freeze never engaged and the eyelid pocket is at risk).
    #[serde(default)]
    pub eye_frozen_vertices: usize,
    #[serde(default)]
    pub elapsed_s: f64,
}

/// A ready-to-run job: which script, which request payload.
#[derive(Debug, Clone)]
pub struct JobSpec {
    /// Python source materialized into the job dir (e.g. [`crate::WRAP_HEAD_SCRIPT`]).
    pub script_source: String,
    pub script_name: String,
    /// Serialized request JSON (already the exact bytes the script will read).
    pub request_json: String,
    pub timeout: Duration,
}

#[derive(Debug)]
pub struct JobOutcome {
    /// Parsed `response.json`.
    pub response: serde_json::Value,
    pub log_path: PathBuf,
    pub elapsed: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("failed to set up job directory: {0}")]
    Setup(#[from] std::io::Error),
    #[error("Blender exited with {code:?}.\n--- log tail ---\n{log_tail}")]
    Failed { code: Option<i32>, log_tail: String },
    #[error("Blender exceeded the {}s timeout and was killed.\n--- log tail ---\n{log_tail}", timeout.as_secs())]
    TimedOut { timeout: Duration, log_tail: String },
    #[error("Blender exited cleanly but wrote no response.json — treating as failure.\n--- log tail ---\n{log_tail}")]
    MissingResponse { log_tail: String },
    #[error("response.json is not valid JSON: {error}.\n--- log tail ---\n{log_tail}")]
    BadResponse {
        error: serde_json::Error,
        log_tail: String,
    },
    #[error("job reported failure: {message}.\n--- log tail ---\n{log_tail}")]
    ScriptError { message: String, log_tail: String },
}

/// Run a job in `job_dir` (created if needed; caller owns cleanup/retention).
pub fn run(blender_exe: &Path, job_dir: &Path, spec: &JobSpec) -> Result<JobOutcome, JobError> {
    std::fs::create_dir_all(job_dir)?;
    let script_path = job_dir.join(&spec.script_name);
    let request_path = job_dir.join("request.json");
    let response_path = job_dir.join("response.json");
    let log_path = job_dir.join("blender.log");
    std::fs::write(&script_path, &spec.script_source)?;
    std::fs::write(&request_path, &spec.request_json)?;
    // A stale response from a previous run must never satisfy this one.
    let _ = std::fs::remove_file(&response_path);

    let log_file = std::fs::File::create(&log_path)?;
    let started = Instant::now();
    let mut child = Command::new(blender_exe)
        .arg("--background")
        .arg("--factory-startup")
        .arg("--python")
        .arg(&script_path)
        .arg("--")
        .arg(&request_path)
        .current_dir(job_dir)
        .stdin(Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file)
        .spawn()?;

    let status = loop {
        match child.try_wait()? {
            Some(status) => break status,
            None => {
                if started.elapsed() > spec.timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(JobError::TimedOut {
                        timeout: spec.timeout,
                        log_tail: log_tail(&log_path),
                    });
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    };
    let elapsed = started.elapsed();

    if !status.success() {
        return Err(JobError::Failed {
            code: status.code(),
            log_tail: log_tail(&log_path),
        });
    }
    let response_text = match std::fs::read_to_string(&response_path) {
        Ok(t) => t,
        Err(_) => {
            return Err(JobError::MissingResponse {
                log_tail: log_tail(&log_path),
            })
        }
    };
    let response: serde_json::Value =
        serde_json::from_str(&response_text).map_err(|error| JobError::BadResponse {
            error,
            log_tail: log_tail(&log_path),
        })?;
    if response.get("status").and_then(|s| s.as_str()) != Some("ok") {
        return Err(JobError::ScriptError {
            message: response
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("(no message)")
                .to_owned(),
            log_tail: log_tail(&log_path),
        });
    }
    Ok(JobOutcome {
        response,
        log_path,
        elapsed,
    })
}

fn log_tail(path: &Path) -> String {
    let Ok(mut f) = std::fs::File::open(path) else {
        return "(no log)".into();
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(LOG_TAIL_BYTES as u64);
    use std::io::Seek;
    let _ = f.seek(std::io::SeekFrom::Start(start));
    let mut buf = String::new();
    let _ = f.read_to_string(&mut buf);
    if buf.is_empty() {
        "(empty log)".into()
    } else {
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shim standing in for blender.exe: a platform script that receives the same
    /// argv shape and behaves per its body. Returns the path to invoke.
    fn shim(dir: &Path, name: &str, body_windows: &str, body_unix: &str) -> PathBuf {
        if cfg!(windows) {
            let p = dir.join(format!("{name}.bat"));
            std::fs::write(&p, body_windows).unwrap();
            p
        } else {
            let p = dir.join(format!("{name}.sh"));
            std::fs::write(&p, format!("#!/bin/sh\n{body_unix}")).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            p
        }
    }

    fn spec() -> JobSpec {
        JobSpec {
            script_source: "# test script".into(),
            script_name: "job.py".into(),
            request_json: r#"{"op":"test"}"#.into(),
            timeout: Duration::from_secs(5),
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bj_{name}_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn happy_path_reads_response() {
        let dir = tmp("ok");
        let exe = shim(
            &dir,
            "blender_ok",
            "@echo off\r\necho booting>&2\r\necho {\"status\":\"ok\",\"vertex_count\":2623} > response.json\r\n",
            "echo booting >&2\nprintf '{\"status\":\"ok\",\"vertex_count\":2623}' > response.json\n",
        );
        let out = run(&exe, &dir.join("job"), &spec()).unwrap();
        assert_eq!(out.response["status"], "ok");
        assert_eq!(out.response["vertex_count"], 2623);
        // request + script were materialized
        assert!(dir.join("job").join("request.json").is_file());
        assert!(dir.join("job").join("job.py").is_file());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nonzero_exit_carries_log_tail() {
        let dir = tmp("fail");
        let exe = shim(
            &dir,
            "blender_fail",
            "@echo off\r\necho Traceback: kaboom\r\nexit /b 3\r\n",
            "echo 'Traceback: kaboom'\nexit 3\n",
        );
        let err = run(&exe, &dir.join("job"), &spec()).unwrap_err();
        match err {
            JobError::Failed { code, log_tail } => {
                assert_eq!(code, Some(3));
                assert!(log_tail.contains("kaboom"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn clean_exit_without_response_is_a_failure() {
        let dir = tmp("noresp");
        let exe = shim(
            &dir,
            "blender_silent",
            "@echo off\r\necho did nothing\r\n",
            "echo did nothing\n",
        );
        assert!(matches!(
            run(&exe, &dir.join("job"), &spec()).unwrap_err(),
            JobError::MissingResponse { .. }
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn script_reported_error_is_surfaced() {
        let dir = tmp("scripterr");
        let exe = shim(
            &dir,
            "blender_err",
            "@echo off\r\necho {\"status\":\"error\",\"message\":\"donor vertex count mismatch\"} > response.json\r\n",
            "printf '{\"status\":\"error\",\"message\":\"donor vertex count mismatch\"}' > response.json\n",
        );
        match run(&exe, &dir.join("job"), &spec()).unwrap_err() {
            JobError::ScriptError { message, .. } => {
                assert!(message.contains("vertex count mismatch"))
            }
            other => panic!("expected ScriptError, got {other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn timeout_kills_and_reports() {
        let dir = tmp("hang");
        let exe = shim(
            &dir,
            "blender_hang",
            "@echo off\r\nping -n 30 127.0.0.1 > nul\r\n",
            "sleep 30\n",
        );
        let mut s = spec();
        s.timeout = Duration::from_millis(500);
        let started = Instant::now();
        assert!(matches!(
            run(&exe, &dir.join("job"), &s).unwrap_err(),
            JobError::TimedOut { .. }
        ));
        assert!(started.elapsed() < Duration::from_secs(10));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stale_response_is_cleared_before_run() {
        let dir = tmp("stale");
        let job = dir.join("job");
        std::fs::create_dir_all(&job).unwrap();
        std::fs::write(job.join("response.json"), r#"{"status":"ok"}"#).unwrap();
        let exe = shim(
            &dir,
            "blender_silent2",
            "@echo off\r\necho ran\r\n",
            "echo ran\n",
        );
        // Shim writes nothing — the stale ok must NOT be accepted.
        assert!(matches!(
            run(&exe, &job, &spec()).unwrap_err(),
            JobError::MissingResponse { .. }
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wrap_request_round_trips() {
        let req = WrapHeadRequest {
            op: "wrap_head".into(),
            donor_obj: "donor/head.obj".into(),
            target_obj: "target/mat_head.obj".into(),
            out_pv: "morphed.pv".into(),
            preview_png: None,
            params: WrapHeadParams {
                eye_center_l: [0.034083, 1.835745, 0.090668],
                eye_center_r: [-0.031242, 1.834532, 0.090979],
                eye_freeze_inner_m: 0.016,
                eye_freeze_outer_m: 0.028,
                boundary_freeze_rings: 8,
                smooth_factor: 0.5,
                smooth_iterations: 6,
                eye_hole_y_range: [1.74, 1.92],
                subdivision_levels: 0,
                expected_vertex_count: 2623,
            },
        };
        let json = serde_json::to_string_pretty(&req).unwrap();
        let back: WrapHeadRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.params.expected_vertex_count, 2623);
        assert!(!json.contains("preview_png"));
    }
}
