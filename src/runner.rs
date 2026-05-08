use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use landlock::{
    path_beneath_rules, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus,
    ABI,
};

pub struct RunResult {
    pub compiler_output: String,
    pub program_output: String,
    pub success: bool,
    pub compile_success: bool,
    pub elapsed_ms: u128,
}

fn unique_id() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    // Mix in the thread ID for extra uniqueness if called rapidly
    let tid = std::thread::current().id();
    let tid_hash = format!("{tid:?}").len() as u64;
    (nanos as u64).wrapping_add(tid_hash).wrapping_mul(6364136223846793005)
}

fn snippet_path() -> String {
    format!("/tmp/rust_pad_{}.rs", unique_id())
}

fn binary_path(snippet: &str) -> String {
    // Strip the .rs extension to get the binary path
    snippet.strip_suffix(".rs").unwrap_or(snippet).to_string()
}

pub fn has_main(code: &str) -> bool {
    code.contains("fn main")
}

pub fn parse_deps(code: &str) -> Vec<String> {
    let mut deps = Vec::new();
    for line in code.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("//! dep:") {
            deps.push(rest.trim().to_string());
        }
    }
    deps
}

fn wrap_in_main(code: &str) -> String {
    if has_main(code) {
        code.to_string()
    } else {
        format!("fn main() {{\n{code}\n}}")
    }
}

/// Apply Landlock filesystem restrictions to the current process.
/// This is meant to be called inside a pre_exec closure (after fork, before exec)
/// so that only the child inherits the restrictions.
///
/// On kernels without Landlock support, this is a best-effort no-op —
/// the child runs unrestricted but we log a warning in the output.
fn apply_landlock_sandbox() -> Result<(), String> {
    let abi = ABI::V2;

    let read_access = AccessFs::from_read(abi);
    let read_write_access = AccessFs::from_all(abi);

    let status = Ruleset::default()
        .handle_access(read_write_access)
        .map_err(|e| format!("landlock handle_access: {e}"))?
        .create()
        .map_err(|e| format!("landlock create: {e}"))?
        // Read-only: /usr, /lib, /lib64, /proc/self
        .add_rules(path_beneath_rules(
            &["/usr", "/lib", "/lib64", "/proc/self"],
            read_access,
        ))
        .map_err(|e| format!("landlock add read rules: {e}"))?
        // Read+write: /tmp (for temp files the snippet might create)
        .add_rules(path_beneath_rules(&["/tmp"], read_write_access))
        .map_err(|e| format!("landlock add write rules: {e}"))?
        .restrict_self()
        .map_err(|e| format!("landlock restrict_self: {e}"))?;

    match status.ruleset {
        RulesetStatus::FullyEnforced | RulesetStatus::PartiallyEnforced => Ok(()),
        RulesetStatus::NotEnforced => {
            // Kernel doesn't support Landlock — best-effort, don't fail
            Ok(())
        }
    }
}

/// Compile only — cleans up temp files before returning.
/// Used by the compile-only (F6) path.
pub fn compile(code: &str) -> RunResult {
    let (result, src, bin) = compile_internal(code);
    cleanup_files(&src, &bin);
    result
}

/// Internal compile that returns the temp file paths so compile_and_run
/// can reuse the binary before cleanup.
fn compile_internal(code: &str) -> (RunResult, String, String) {
    let wrapped = wrap_in_main(code);
    let deps = parse_deps(code);

    let src = snippet_path();
    let bin = binary_path(&src);

    if let Err(e) = std::fs::write(&src, &wrapped) {
        return (
            RunResult {
                compiler_output: format!("Failed to write snippet: {e}"),
                program_output: String::new(),
                success: false,
                compile_success: false,
                elapsed_ms: 0,
            },
            src,
            bin,
        );
    }

    let start = Instant::now();

    let mut cmd = Command::new("rustc");
    cmd.arg(&src).arg("-o").arg(&bin).arg("--edition").arg("2021");

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            let _ = std::fs::remove_file(&src);
            return (
                RunResult {
                    compiler_output: format!("Failed to run rustc: {e}"),
                    program_output: String::new(),
                    success: false,
                    compile_success: false,
                    elapsed_ms: start.elapsed().as_millis(),
                },
                src,
                bin,
            );
        }
    };

    let elapsed = start.elapsed().as_millis();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let compile_success = output.status.success();

    let dep_note = if !deps.is_empty() {
        format!(
            "\n[note: deps detected: {} — cargo integration is a future feature]",
            deps.join(", ")
        )
    } else {
        String::new()
    };

    (
        RunResult {
            compiler_output: format!("{stderr}{dep_note}"),
            program_output: String::new(),
            success: compile_success,
            compile_success,
            elapsed_ms: elapsed,
        },
        src,
        bin,
    )
}

pub fn compile_and_run(code: &str) -> RunResult {
    let (mut result, src, bin) = compile_internal(code);
    if !result.compile_success {
        let _ = std::fs::remove_file(&src);
        return result;
    }

    let start = Instant::now();
    let timeout = Duration::from_secs(10);

    let bin_clone = bin.clone();
    let mut child = match unsafe {
        Command::new(&bin)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .pre_exec(move || {
                // Apply Landlock sandbox to the child process.
                // This runs after fork() but before exec(), so the parent
                // (rust-pad itself) is NOT restricted.
                if let Err(e) = apply_landlock_sandbox() {
                    // Write the error to stderr so the user sees it
                    eprintln!("[sandbox warning: {e}]");
                    // Don't fail — run unsandboxed as fallback on older kernels
                }
                Ok(())
            })
            .spawn()
    } {
        Ok(c) => c,
        Err(e) => {
            result.program_output = format!("Failed to execute: {e}");
            result.success = false;
            cleanup_files(&src, &bin_clone);
            return result;
        }
    };

    let run_output = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = child.wait_with_output().unwrap_or_else(|_| std::process::Output {
                    status,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                });
                break out;
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    result.program_output = format!("[killed: exceeded {timeout:?} timeout]");
                    result.success = false;
                    result.elapsed_ms += start.elapsed().as_millis();
                    cleanup_files(&src, &bin);
                    return result;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                result.program_output = format!("Failed to wait: {e}");
                result.success = false;
                cleanup_files(&src, &bin);
                return result;
            }
        }
    };

    let run_elapsed = start.elapsed().as_millis();

    let stdout = String::from_utf8_lossy(&run_output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run_output.stderr).to_string();

    result.program_output = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        format!("[stderr]\n{stderr}")
    } else {
        format!("{stdout}\n[stderr]\n{stderr}")
    };

    result.success = run_output.status.success();
    result.elapsed_ms += run_elapsed;

    let exit_str = match run_output.status.code() {
        Some(0) => String::new(),
        Some(c) => format!("\n[exit code: {c}]"),
        None => "\n[killed by signal]".to_string(),
    };
    result.program_output.push_str(&exit_str);

    cleanup_files(&src, &bin);
    result
}

/// Remove temp source and binary files after execution
fn cleanup_files(src: &str, bin: &str) {
    let _ = std::fs::remove_file(src);
    let _ = std::fs::remove_file(bin);
}
