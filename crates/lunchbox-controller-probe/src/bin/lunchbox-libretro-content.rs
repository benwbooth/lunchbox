use anyhow::{Context, Result, ensure};
use clap::Parser;
use lunchbox_controller_probe::content_inspection::Request;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(about = "Inspect real-content controller descriptors in a supervised trusted core")]
struct Args {
    #[arg(long)]
    request: PathBuf,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300))]
    timeout_seconds: u64,
    #[arg(long, hide = true, requires = "worker_result")]
    worker: bool,
    #[arg(long, hide = true, requires = "worker")]
    worker_result: Option<PathBuf>,
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "Inspection file exceeds size limit"
    );
    Ok(bytes)
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", deny_unknown_fields)]
enum Outcome {
    Success { report: serde_json::Value },
    Failure { error: String },
}

struct Worker(std::process::Child);
impl Drop for Worker {
    fn drop(&mut self) {
        // Wait after kill so the private workspace is never removed while the
        // native worker may still be writing into it.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let request: Request = serde_json::from_slice(&read_bounded(&args.request, 1024 * 1024)?)?;
    request.validate()?;
    if args.worker {
        let outcome = match request.inspect() {
            Ok(report) => Outcome::Success {
                report: serde_json::to_value(report)?,
            },
            Err(error) => Outcome::Failure {
                error: format!("{error:#}").chars().take(16384).collect(),
            },
        };
        let bytes = serde_json::to_vec(&outcome)?;
        ensure!(
            bytes.len() <= 8 * 1024 * 1024,
            "Inspection report exceeds size limit"
        );
        let mut result = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(
                args.worker_result
                    .context("Missing private worker result path")?,
            )?;
        result.write_all(&bytes)?;
        return Ok(());
    }

    // The supervisor owns the entire temporary tree, including the child's
    // tempfile root. It survives native crashes and is cleaned after reaping.
    let workspace = tempfile::tempdir()?;
    let private_request = workspace.path().join("request.json");
    let result_path = workspace.path().join("result.json");
    let temporary_root = workspace.path().join("temporary");
    std::fs::create_dir(&temporary_root)?;
    std::fs::write(&private_request, serde_json::to_vec(&request)?)?;
    let mut worker = Worker(
        Command::new(std::env::current_exe()?)
            .arg("--worker")
            .arg("--request")
            .arg(&private_request)
            .arg("--worker-result")
            .arg(&result_path)
            .env("TMPDIR", &temporary_root)
            .env("TMP", &temporary_root)
            .env("TEMP", &temporary_root)
            .current_dir(workspace.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Starting isolated content-inspection worker")?,
    );
    let deadline = Instant::now() + Duration::from_secs(args.timeout_seconds);
    let status = loop {
        if let Some(status) = worker.0.try_wait()? {
            break status;
        }
        ensure!(
            Instant::now() < deadline,
            "Trusted core inspection exceeded its wall-time limit"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    ensure!(
        status.success(),
        "Content-inspection worker failed: {status}"
    );
    let outcome: Outcome = serde_json::from_slice(&read_bounded(&result_path, 8 * 1024 * 1024)?)?;
    match outcome {
        Outcome::Success { report } => {
            let report = request.parse_report(&serde_json::to_vec(&report)?)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Outcome::Failure { error } => anyhow::bail!("Content inspection failed: {error}"),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
