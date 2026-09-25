//! Optional, app-managed Ollama container. The RetroArch bridge and OCR stay
//! in Lunchbox; Docker provides the otherwise separately installed LLM server.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::path::Path;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

const CONTAINER: &str = "lunchbox-ollama";
const VOLUME: &str = "lunchbox-ollama-models";
const OWNER_LABEL: &str = "io.github.benwbooth.lunchbox.translation";
const VERSION: &str = "0.32.3";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compute {
    Nvidia,
    Rocm,
}

impl Compute {
    fn detected() -> Result<Self> {
        if cfg!(all(target_os = "linux", target_arch = "x86_64"))
            && Path::new("/dev/kfd").exists()
            && Path::new("/dev/dri").exists()
        {
            Ok(Self::Rocm)
        } else if cfg!(any(target_os = "linux", target_os = "windows"))
            && Command::new("nvidia-smi")
                .arg("--query-gpu=name")
                .arg("--format=csv,noheader")
                .output()
                .is_ok_and(|output| output.status.success())
        {
            Ok(Self::Nvidia)
        } else {
            bail!(
                "No supported Docker GPU backend was detected. This Ollama container requires AMD ROCm on Linux or NVIDIA GPU passthrough on Linux/Windows; CPU translation is disabled. Apple Silicon needs a Metal-capable host service, not a Linux container."
            )
        }
    }

    fn image(self) -> String {
        match self {
            Self::Rocm => format!("ollama/ollama:{VERSION}-rocm"),
            Self::Nvidia => format!("ollama/ollama:{VERSION}"),
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA GPU",
            Self::Rocm => "AMD ROCm GPU",
        }
    }
}

fn docker_command() -> Command {
    // Flatpak has network access and permission to ask its host to run Docker,
    // but intentionally does not carry the Docker CLI or daemon socket inside.
    if std::env::var_os("FLATPAK_ID").is_some() {
        let mut command = Command::new("flatpak-spawn");
        command.args(["--host", "docker"]);
        command
    } else {
        Command::new("docker")
    }
}

fn docker(args: &[&str]) -> Result<Output> {
    docker_command()
        .args(args)
        .output()
        .context("running Docker; install and start Docker Engine or Docker Desktop")
}

fn docker_success(args: &[&str]) -> Result<String> {
    let output = docker(args)?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Docker failed: {}",
            message.trim().chars().take(600).collect::<String>()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn container_state() -> Result<Option<bool>> {
    let output = docker(&["inspect", CONTAINER])?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        if error.to_ascii_lowercase().contains("no such object") {
            return Ok(None);
        }
        bail!("Could not inspect Docker container: {error}");
    }
    let records: Value = serde_json::from_slice(&output.stdout)?;
    let record = records
        .as_array()
        .and_then(|records| records.first())
        .context("Docker returned no container details")?;
    ensure!(
        record["Config"]["Labels"][OWNER_LABEL] == "ollama",
        "Docker container {CONTAINER} already exists but is not managed by Lunchbox"
    );
    Ok(Some(record["State"]["Running"].as_bool().unwrap_or(false)))
}

fn ensure_model_volume() -> Result<()> {
    let output = docker(&["volume", "inspect", VOLUME])?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        ensure!(
            error.to_ascii_lowercase().contains("no such volume"),
            "Could not inspect Docker volume: {error}"
        );
        docker_success(&[
            "volume",
            "create",
            "--label",
            &format!("{OWNER_LABEL}=ollama"),
            VOLUME,
        ])?;
        return Ok(());
    }
    let records: Value = serde_json::from_slice(&output.stdout)?;
    let record = records
        .as_array()
        .and_then(|records| records.first())
        .context("Docker returned no volume details")?;
    ensure!(
        record["Labels"][OWNER_LABEL] == "ollama",
        "Docker volume {VOLUME} already exists but is not managed by Lunchbox"
    );
    Ok(())
}

fn port_is_taken() -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 11434)),
        Duration::from_millis(400),
    )
    .is_ok()
}

fn ollama_is_ready() -> bool {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(1)))
        .build()
        .into();
    agent
        .get("http://127.0.0.1:11434/api/tags")
        .call()
        .is_ok_and(|response| response.status().as_u16() == 200)
}

fn run_args(compute: Compute) -> Vec<String> {
    let mut args = vec![
        "run".into(),
        "--detach".into(),
        "--pull=missing".into(),
        "--name".into(),
        CONTAINER.into(),
        "--label".into(),
        format!("{OWNER_LABEL}=ollama"),
        "--restart".into(),
        "unless-stopped".into(),
        "--publish".into(),
        "127.0.0.1:11434:11434".into(),
        "--mount".into(),
        format!("type=volume,source={VOLUME},target=/root/.ollama"),
    ];
    match compute {
        Compute::Rocm => args.extend(["--device=/dev/kfd".into(), "--device=/dev/dri".into()]),
        Compute::Nvidia => args.extend(["--gpus".into(), "all".into()]),
    }
    args.push(compute.image());
    args
}

pub fn setup(cancelled: &AtomicBool, mut progress: impl FnMut(String)) -> Result<Compute> {
    ensure!(!cancelled.load(Ordering::Relaxed), "setup cancelled");
    crate::translation::preflight_gpu_ocr()?;
    docker_success(&["info", "--format", "{{.ServerVersion}}"])?;
    let state = container_state()?;
    if state != Some(true) && port_is_taken() {
        bail!(
            "Another service is using 127.0.0.1:11434. Stop the native Ollama service before setting up the Docker backend."
        );
    }
    let compute = Compute::detected()?;
    match state {
        Some(true) => progress(format!(
            "Using the existing Lunchbox Docker container ({})",
            compute.description()
        )),
        Some(false) => {
            progress("Starting the existing Lunchbox Docker container…".into());
            docker_success(&["start", CONTAINER])?;
        }
        None => {
            ensure!(!cancelled.load(Ordering::Relaxed), "setup cancelled");
            progress(format!(
                "Downloading and starting Ollama in Docker ({})…",
                compute.description()
            ));
            ensure_model_volume()?;
            let args = run_args(compute);
            let references = args.iter().map(String::as_str).collect::<Vec<_>>();
            docker_success(&references)?;
        }
    }
    for _ in 0..40 {
        if ollama_is_ready() {
            return Ok(compute);
        }
        ensure!(!cancelled.load(Ordering::Relaxed), "setup cancelled");
        std::thread::sleep(Duration::from_millis(500));
    }
    bail!("Ollama did not open its local port; inspect the {CONTAINER} container logs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_container_binds_loopback_and_persists_models() {
        let args = run_args(Compute::Rocm).join(" ");
        assert!(args.contains("127.0.0.1:11434:11434"));
        assert!(args.contains("type=volume,source=lunchbox-ollama-models,target=/root/.ollama"));
        assert!(args.contains("io.github.benwbooth.lunchbox.translation=ollama"));
        assert!(run_args(Compute::Rocm).contains(&"--device=/dev/kfd".to_owned()));
        assert!(run_args(Compute::Nvidia).contains(&"all".to_owned()));
    }
}
