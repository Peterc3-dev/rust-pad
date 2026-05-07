use std::process::Command;
use std::time::{Duration, Instant};

pub struct RunResult {
    pub compiler_output: String,
    pub program_output: String,
    pub success: bool,
    pub compile_success: bool,
    pub elapsed_ms: u128,
}

const SNIPPET_PATH: &str = "/tmp/rust_pad_snippet.rs";
const BINARY_PATH: &str = "/tmp/rust_pad_snippet";

pub fn has_main(code: &str) -> bool {
    // Simple check: does the code contain `fn main`
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

pub fn compile(code: &str) -> RunResult {
    let wrapped = wrap_in_main(code);
    let deps = parse_deps(code);

    if let Err(e) = std::fs::write(SNIPPET_PATH, &wrapped) {
        return RunResult {
            compiler_output: format!("Failed to write snippet: {e}"),
            program_output: String::new(),
            success: false,
            compile_success: false,
            elapsed_ms: 0,
        };
    }

    let start = Instant::now();

    let mut cmd = Command::new("rustc");
    cmd.arg(SNIPPET_PATH)
        .arg("-o")
        .arg(BINARY_PATH)
        .arg("--edition")
        .arg("2021");

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return RunResult {
                compiler_output: format!("Failed to run rustc: {e}"),
                program_output: String::new(),
                success: false,
                compile_success: false,
                elapsed_ms: start.elapsed().as_millis(),
            };
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

    RunResult {
        compiler_output: format!("{stderr}{dep_note}"),
        program_output: String::new(),
        success: compile_success,
        compile_success,
        elapsed_ms: elapsed,
    }
}

pub fn compile_and_run(code: &str) -> RunResult {
    let mut result = compile(code);
    if !result.compile_success {
        return result;
    }

    let start = Instant::now();
    let timeout = Duration::from_secs(10);

    let mut child = match Command::new(BINARY_PATH)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            result.program_output = format!("Failed to execute: {e}");
            result.success = false;
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
                    return result;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                result.program_output = format!("Failed to wait: {e}");
                result.success = false;
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

    result
}
