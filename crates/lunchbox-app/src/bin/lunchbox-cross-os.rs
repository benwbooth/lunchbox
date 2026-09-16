//! lunchbox-cross-os: run the feature test suites on Linux, macOS, and
//! Windows, then report one combined outcome.
//!
//! The macOS leg runs over SSH on m1.local and the Windows leg over SSH on
//! the audit VM; the Linux leg runs locally (launch this under
//! `nix develop` so the Qt environment is present). Legs run sequentially:
//! full workspace builds are memory-hungry and the Windows VM has died
//! under concurrent host build load before.
//!
//! Each leg runs `cargo test --locked -p lunchbox-app
//! -p lunchbox-controller-probe` with libtest JSON output, which this tool
//! aggregates into a single report. Emulator-installation oracles stay
//! `#[ignore]`d behind their payload gates, exactly as in CI.

use anyhow::{Context, Result};
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_SECS: u64 = 5400;
const TEST_PACKAGES: [&str; 2] = ["lunchbox-app", "lunchbox-controller-probe"];

#[derive(Debug, Clone)]
struct Leg {
    /// Short name used in the report (`linux`, `macos`, `windows`).
    name: &'static str,
    /// How to invoke the test command.
    target: Target,
    /// Remote working checkout (informational; baked into the command).
    checkout: &'static str,
}

#[derive(Debug, Clone)]
enum Target {
    Local,
    /// `ssh [ssh_opts...] target <command>`
    Ssh {
        target: String,
        ssh_opts: Vec<String>,
        command: String,
    },
}

#[derive(Debug, Default, serde::Serialize)]
struct LegReport {
    name: String,
    target: String,
    checkout: String,
    status: String,
    passed: u64,
    failed: u64,
    ignored: u64,
    failures: Vec<String>,
    duration_secs: u64,
    detail: String,
}

#[derive(Debug, Default, serde::Serialize)]
struct Report {
    overall: String,
    legs: Vec<LegReport>,
}

fn mac_command() -> String {
    [
        "cd ~/lunchbox-mac",
        "git pull --ff-only -q",
        "CMAKE_PREFIX_PATH=$HOME/qt/6.8.3/macos rustup run stable cargo test --locked -p lunchbox-app -p lunchbox-controller-probe -- --format json --report-time",
    ]
    .join(" && ")
}

fn windows_command() -> String {
    // C:\\tools\\run-lunchbox-tests.bat is provisioned on the VM (outside
    // the repo): it loads the VS slug, pulls main, and runs the suites.
    "C:\\tools\\run-lunchbox-tests.bat".to_owned()
}

fn local_command() -> String {
    format!(
        "cargo test --locked {} -- --format json --report-time",
        TEST_PACKAGES
            .iter()
            .map(|package| format!("-p {package}"))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn run_with_timeout(mut command: Command, timeout: Duration) -> Result<std::process::Output> {
    use std::sync::mpsc;
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().context("spawning test command")?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    match receiver.recv_timeout(timeout) {
        Ok(output) => output.context("reading test output"),
        Err(_) => anyhow::bail!("leg timed out after {}s", timeout.as_secs()),
    }
}

fn summarize_libtest_json(output: &[u8]) -> (u64, u64, u64, Vec<String>) {
    let mut passed = 0;
    let mut failed = 0;
    let mut ignored = 0;
    let mut failures = Vec::new();
    for line in output.lines().map_while(Result::ok) {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if event.get("type").and_then(|value| value.as_str()) != Some("test") {
            continue;
        }
        match event.get("event").and_then(|value| value.as_str()) {
            Some("ok") => passed += 1,
            Some("failed") => {
                failed += 1;
                if let Some(name) = event.get("name").and_then(|value| value.as_str()) {
                    failures.push(name.to_owned());
                }
            }
            Some("ignored") => ignored += 1,
            _ => {}
        }
    }
    (passed, failed, ignored, failures)
}

fn run_leg(leg: &Leg, timeout: Duration, verbose: bool) -> LegReport {
    let started = Instant::now();
    let mut report = LegReport {
        name: leg.name.to_owned(),
        checkout: leg.checkout.to_owned(),
        ..LegReport::default()
    };
    let (target_text, mut command) = match &leg.target {
        Target::Local => {
            report.target = "local".to_owned();
            let mut command = Command::new("bash");
            command.args(["-c", &local_command()]);
            ("local".to_owned(), command)
        }
        Target::Ssh {
            target,
            ssh_opts,
            command: remote,
        } => {
            report.target = format!("ssh {target}");
            let mut command = Command::new("ssh");
            command.args(ssh_opts).arg(target).arg(remote);
            (format!("ssh {target}"), command)
        }
    };
    match run_with_timeout(command, timeout) {
        Ok(output) => {
            let combined = [output.stdout.clone(), output.stderr.clone()].concat();
            let (passed, failed, ignored, failures) = summarize_libtest_json(&combined);
            report.passed = passed;
            report.failed = failed;
            report.ignored = ignored;
            report.failures = failures;
            report.duration_secs = started.elapsed().as_secs();
            if !output.status.success() && failed == 0 {
                report.status = "error".to_owned();
                report.detail = format!(
                    "command exited {} with no parsed failures; tail: {}",
                    output.status,
                    String::from_utf8_lossy(&combined)
                        .lines()
                        .rev()
                        .take(5)
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
            } else if failed > 0 {
                report.status = "failed".to_owned();
            } else if passed == 0 {
                report.status = "error".to_owned();
                report.detail = format!(
                    "no test events parsed; tail: {}",
                    String::from_utf8_lossy(&combined)
                        .lines()
                        .rev()
                        .take(5)
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
            } else {
                report.status = "passed".to_owned();
            }
            if verbose {
                println!(
                    "[{target_text}] {}: passed={passed} failed={failed} ignored={ignored} ({}s)",
                    report.status, report.duration_secs
                );
            }
        }
        Err(error) => {
            report.status = "error".to_owned();
            report.detail = format!("{error:#}");
            report.duration_secs = started.elapsed().as_secs();
        }
    }
    report
}

fn print_usage(program: &str) {
    println!(
        "usage: {program} [--report PATH] [--timeout-secs N] [--skip-macos] [--skip-windows] [--mac-target SSH] [--win-target SSH] [--verbose]"
    );
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let program = args.first().cloned().unwrap_or_default();
    let mut report_path: Option<PathBuf> = None;
    let mut timeout = Duration::from_secs(DEFAULT_TIMEOUT_SECS);
    let mut skip_macos = false;
    let mut skip_windows = false;
    let mut mac_target = "bbooth@m1.local".to_owned();
    let mut win_target = "oracle@192.168.122.207".to_owned();
    let mut verbose = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--report" => {
                index += 1;
                report_path = args.get(index).map(PathBuf::from);
            }
            "--timeout-secs" => {
                index += 1;
                timeout = Duration::from_secs(
                    args.get(index)
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(DEFAULT_TIMEOUT_SECS),
                );
            }
            "--skip-macos" => skip_macos = true,
            "--skip-windows" => skip_windows = true,
            "--mac-target" => {
                index += 1;
                if let Some(target) = args.get(index) {
                    mac_target = target.clone();
                }
            }
            "--win-target" => {
                index += 1;
                if let Some(target) = args.get(index) {
                    win_target = target.clone();
                }
            }
            "--verbose" => verbose = true,
            "--help" | "-h" => {
                print_usage(&program);
                return Ok(());
            }
            other => {
                anyhow::bail!("unknown argument {other}");
            }
        }
        index += 1;
    }
    let ssh_opts = vec![
        "-o".to_owned(),
        "BatchMode=yes".to_owned(),
        "-o".to_owned(),
        "ConnectTimeout=20".to_owned(),
        "-o".to_owned(),
        "StrictHostKeyChecking=accept-new".to_owned(),
    ];
    let mut legs = vec![Leg {
        name: "linux",
        target: Target::Local,
        checkout: ".",
    }];
    if !skip_macos {
        legs.push(Leg {
            name: "macos",
            target: Target::Ssh {
                target: mac_target,
                ssh_opts: ssh_opts.clone(),
                command: mac_command(),
            },
            checkout: "~/lunchbox-mac",
        });
    }
    if !skip_windows {
        legs.push(Leg {
            name: "windows",
            target: Target::Ssh {
                target: win_target,
                ssh_opts,
                command: windows_command(),
            },
            checkout: "C:\\lunchbox",
        });
    }
    let mut report = Report::default();
    for leg in &legs {
        println!("=== leg: {} ({}) ===", leg.name, leg.checkout);
        report.legs.push(run_leg(leg, timeout, verbose));
    }
    report.overall = if report.legs.iter().all(|leg| leg.status == "passed") {
        "passed".to_owned()
    } else {
        "failed".to_owned()
    };
    println!("=== overall: {} ===", report.overall);
    for leg in &report.legs {
        println!(
            "{}: {} (passed={} failed={} ignored={} {}s)",
            leg.name, leg.status, leg.passed, leg.failed, leg.ignored, leg.duration_secs
        );
        for failure in &leg.failures {
            println!("  FAILED {failure}");
        }
        if !leg.detail.is_empty() {
            println!("  detail: {}", leg.detail);
        }
    }
    if let Some(path) = report_path {
        std::fs::write(&path, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("writing {}", path.display()))?;
        println!("report: {}", path.display());
    }
    if report.overall != "passed" {
        std::process::exit(1);
    }
    Ok(())
}
