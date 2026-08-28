use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::params_from_iter;

use crate::exo_install::{ExoCollection, PreparedInstall};

const DEFAULT_SCUMMVM_CONFIG: &str = r#"[scummvm]
filtering=false
autosave_period=300
mute=false
speech_volume=192
native_mt32=false
mt32_device=mt32
kbdmouse_speed=3
talkspeed=60
midi_gain=100
subtitles=false
multi_midi=false
fullscreen=false
updates_check=2628000
gui_browser_show_hidden=false
gm_device=null
sfx_volume=192
music_volume=192
speech_mute=false
music_driver=auto
opl_driver=auto
aspect_ratio=false
gui_theme=SCUMMMODERN
enable_gs=false
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostPlatform {
    Linux,
    Windows,
    MacOs,
}

impl HostPlatform {
    fn current() -> Result<Self> {
        if cfg!(target_os = "linux") {
            Ok(Self::Linux)
        } else if cfg!(target_os = "windows") {
            Ok(Self::Windows)
        } else if cfg!(target_os = "macos") {
            Ok(Self::MacOs)
        } else {
            bail!("Lunchbox emulator discovery does not support this host operating system")
        }
    }

    fn catalog_slug(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Windows => "windows",
            Self::MacOs => "macos",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EmulatorDefinition {
    id: String,
    name: String,
    packages: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmulatorExecutable {
    Native(PathBuf),
    Flatpak { command: PathBuf, app_id: String },
}

impl EmulatorExecutable {
    pub fn summary(&self) -> String {
        match self {
            Self::Native(path) => path.display().to_string(),
            Self::Flatpak { app_id, .. } => format!("Flatpak · {app_id}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmulatorChoice {
    pub id: String,
    pub name: String,
    pub executable: EmulatorExecutable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DosboxExceptionPlan {
    copy_mt32_roms: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxDosboxExceptionPlan {
    launch_config_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxScummvmExceptionPlan {
    config_path: PathBuf,
    game_path: PathBuf,
    game_id: String,
    extra_args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EightySixBoxPlan {
    config_name: &'static str,
    parent_vhd_name: &'static str,
    child_vhd_name: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PcBoxPlan {
    config_name: &'static str,
    parent_vhd_name: &'static str,
    child_vhd_name: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Win9xLauncherKind {
    DosboxX,
    EightySixBox(EightySixBoxPlan),
    PcBox(PcBoxPlan),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PreparedLaunchKind {
    Dosbox {
        config_path: PathBuf,
        shared_options_path: Option<PathBuf>,
        copy_mt32_roms: bool,
    },
    ScummVm(LinuxScummvmExceptionPlan),
    Win9xDosboxX {
        config_path: PathBuf,
        shared_options_path: PathBuf,
    },
    EightySixBox(EightySixBoxPlan),
    PcBox(PcBoxPlan),
}

impl PreparedLaunchKind {
    fn required_emulator_names(&self) -> &'static [&'static str] {
        match self {
            Self::Dosbox { .. } => &["DOSBox Staging", "DOSBox-X"],
            Self::ScummVm(_) => &["ScummVM"],
            Self::Win9xDosboxX { .. } => &["DOSBox-X"],
            Self::EightySixBox(_) => &["86Box"],
            Self::PcBox(_) => &["PCBox", "86Box"],
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Dosbox { .. } => "DOSBox prepared install",
            Self::ScummVm(_) => "ScummVM exception launch",
            Self::Win9xDosboxX { .. } => "DOSBox-X Windows 9x profile",
            Self::EightySixBox(_) => "86Box Windows 9x profile",
            Self::PcBox(_) => "PCBox-compatible Windows 9x profile",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchAvailability {
    pub emulator: Option<EmulatorChoice>,
    pub requirement: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchPlan {
    pub emulator_name: String,
    pub program: PathBuf,
    pub arguments: Vec<OsString>,
    pub current_directory: PathBuf,
    pub cleanup_paths: Vec<PathBuf>,
}

impl LaunchPlan {
    pub fn command_summary(&self) -> String {
        let mut parts = vec![self.program.display().to_string()];
        parts.extend(
            self.arguments
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned()),
        );
        parts.join(" ")
    }
}

pub fn inspect_launch_availability(
    prepared: &PreparedInstall,
    catalog_database: &Path,
) -> Result<LaunchAvailability> {
    let kind = classify_prepared_install(prepared)?;
    let host = HostPlatform::current()?;
    let definitions =
        load_emulator_definitions(catalog_database, host, kind.required_emulator_names())?;
    let flatpak_apps = installed_flatpak_apps(host);
    let path_entries = executable_search_directories();
    let emulator = definitions
        .iter()
        .find_map(|definition| discover_definition(definition, host, &path_entries, &flatpak_apps));
    let required = kind.required_emulator_names().join(" or ");
    let detail = if let Some(choice) = &emulator {
        format!(
            "{} is ready with {} ({})",
            kind.description(),
            choice.name,
            choice.executable.summary()
        )
    } else {
        format!(
            "{} needs an installed standalone {required}",
            kind.description()
        )
    };
    Ok(LaunchAvailability {
        emulator,
        requirement: required,
        detail,
    })
}

pub fn build_launch_plan(
    prepared: &PreparedInstall,
    catalog_database: &Path,
) -> Result<LaunchPlan> {
    let kind = classify_prepared_install(prepared)?;
    let availability = inspect_launch_availability(prepared, catalog_database)?;
    let emulator = availability.emulator.ok_or_else(|| {
        anyhow!(
            "No compatible emulator is installed. Install {} and refresh detection.",
            availability.requirement
        )
    })?;
    build_plan_for_choice(prepared, kind, emulator)
}

pub fn spawn_launch_plan(plan: &LaunchPlan) -> Result<Child> {
    let mut command = Command::new(&plan.program);
    command
        .args(&plan.arguments)
        .current_dir(&plan.current_directory)
        .stdin(Stdio::null());
    command.spawn().with_context(|| {
        format!(
            "starting {} with {}",
            plan.emulator_name,
            plan.command_summary()
        )
    })
}

pub fn cleanup_after_launch(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn classify_prepared_install(prepared: &PreparedInstall) -> Result<PreparedLaunchKind> {
    match prepared.collection {
        ExoCollection::Dos | ExoCollection::Win3x => classify_dosbox_install(prepared),
        ExoCollection::Win9x => classify_win9x_install(prepared),
    }
}

fn classify_dosbox_install(prepared: &PreparedInstall) -> Result<PreparedLaunchKind> {
    let metadata_dir = prepared.launch_config_path.parent().with_context(|| {
        format!(
            "prepared config {} has no parent directory",
            prepared.launch_config_path.display()
        )
    })?;
    let linux_exception = metadata_dir.join("exception.bsh");
    let windows_exception = metadata_dir.join("exception.bat");
    let mut config_path = prepared.launch_config_path.clone();
    let mut copy_mt32_roms = false;

    if linux_exception.is_file() {
        let contents = fs::read_to_string(&linux_exception).with_context(|| {
            format!(
                "reading Linux eXo exception launcher {}",
                linux_exception.display()
            )
        })?;
        if let Ok(scummvm) = parse_linux_scummvm_exception_script(&contents) {
            return Ok(PreparedLaunchKind::ScummVm(scummvm));
        }
        let dosbox = parse_linux_dosbox_exception_script(&contents).with_context(|| {
            format!(
                "unsupported Linux eXo exception launcher {}",
                linux_exception.display()
            )
        })?;
        config_path = metadata_dir.join(dosbox.launch_config_name);
        if !config_path.is_file() {
            bail!(
                "Linux eXo exception launcher {} refers to missing config {}",
                linux_exception.display(),
                config_path.display()
            );
        }
    } else if windows_exception.is_file() {
        let contents = fs::read_to_string(&windows_exception).with_context(|| {
            format!(
                "reading eXo exception launcher {}",
                windows_exception.display()
            )
        })?;
        copy_mt32_roms = parse_dosbox_exception_script(&contents)
            .with_context(|| {
                format!(
                    "unsupported eXo exception launcher {}",
                    windows_exception.display()
                )
            })?
            .copy_mt32_roms;
    }

    let prefers_linux_options = config_path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.to_ascii_lowercase().ends_with("_linux.conf"));
    let option_names = if prefers_linux_options {
        ["options_linux.conf", "options.conf"]
    } else {
        ["options.conf", "options_linux.conf"]
    };
    let shared_options_path = option_names
        .into_iter()
        .map(|name| prepared.install_root.join("emulators/dosbox").join(name))
        .find(|path| path.is_file());

    Ok(PreparedLaunchKind::Dosbox {
        config_path,
        shared_options_path,
        copy_mt32_roms,
    })
}

fn classify_win9x_install(prepared: &PreparedInstall) -> Result<PreparedLaunchKind> {
    let metadata_dir = prepared.launch_config_path.parent().with_context(|| {
        format!(
            "prepared config {} has no parent directory",
            prepared.launch_config_path.display()
        )
    })?;
    match detect_win9x_launcher_kind(metadata_dir)? {
        Win9xLauncherKind::DosboxX => {
            let shared_options_path = prepared
                .install_root
                .join("emulators/dosbox/options9x.conf");
            if !shared_options_path.is_file() {
                bail!(
                    "prepared eXoWin9x install is missing {}",
                    shared_options_path.display()
                );
            }
            Ok(PreparedLaunchKind::Win9xDosboxX {
                config_path: prepared.launch_config_path.clone(),
                shared_options_path,
            })
        }
        Win9xLauncherKind::EightySixBox(plan) => Ok(PreparedLaunchKind::EightySixBox(plan)),
        Win9xLauncherKind::PcBox(plan) => Ok(PreparedLaunchKind::PcBox(plan)),
    }
}

fn load_emulator_definitions(
    database: &Path,
    host: HostPlatform,
    names: &[&str],
) -> Result<Vec<EmulatorDefinition>> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let connection = crate::catalog::open_read_only(database, "Lunchbox emulator catalog")?;
    let placeholders = (0..names.len()).map(|_| "?").collect::<Vec<_>>().join(",");
    let mut statement = connection.prepare(&format!(
        "SELECT e.id, e.name
         FROM emulators e
         JOIN emulator_host_systems h ON h.emulator_id=e.id
         WHERE e.name COLLATE NOCASE IN ({placeholders}) AND h.host_system_slug=?
         ORDER BY e.name COLLATE NOCASE"
    ))?;
    let mut values = names
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    values.push(host.catalog_slug().to_owned());
    let rows = statement.query_map(params_from_iter(values), |row| {
        Ok(EmulatorDefinition {
            id: row.get(0)?,
            name: row.get(1)?,
            packages: BTreeMap::new(),
        })
    })?;
    let mut by_name = rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .map(|definition| (definition.name.to_ascii_lowercase(), definition))
        .collect::<BTreeMap<_, _>>();

    for definition in by_name.values_mut() {
        let mut packages = connection.prepare(
            "SELECT manager, package_id FROM emulator_packages
             WHERE emulator_id=?1 ORDER BY manager, package_id",
        )?;
        let rows = packages.query_map([&definition.id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (manager, package_id) = row?;
            definition
                .packages
                .entry(manager)
                .or_default()
                .push(package_id);
        }
    }

    Ok(names
        .iter()
        .filter_map(|name| by_name.remove(&name.to_ascii_lowercase()))
        .collect())
}

fn discover_definition(
    definition: &EmulatorDefinition,
    host: HostPlatform,
    path_entries: &[PathBuf],
    flatpak_apps: &BTreeSet<String>,
) -> Option<EmulatorChoice> {
    let mut native_directories = platform_install_directories(definition, host);
    native_directories.extend_from_slice(path_entries);
    let executable = executable_names(&definition.name, host)
        .into_iter()
        .find_map(|name| find_executable_in_paths(&name, &native_directories))
        .map(EmulatorExecutable::Native)
        .or_else(|| discover_macos_application(definition, host))
        .or_else(|| {
            let flatpak_command = find_executable_in_paths("flatpak", path_entries)?;
            definition
                .packages
                .get("flatpak")?
                .iter()
                .find_map(|app_id| {
                    flatpak_apps
                        .contains(app_id)
                        .then(|| EmulatorExecutable::Flatpak {
                            command: flatpak_command.clone(),
                            app_id: app_id.clone(),
                        })
                })
        })?;
    Some(EmulatorChoice {
        id: definition.id.clone(),
        name: definition.name.clone(),
        executable,
    })
}

fn platform_install_directories(
    definition: &EmulatorDefinition,
    host: HostPlatform,
) -> Vec<PathBuf> {
    if host != HostPlatform::Windows {
        return Vec::new();
    }
    let mut directories = Vec::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = env::var_os(variable).filter(|value| !value.is_empty()) {
            directories.push(PathBuf::from(root).join(&definition.name));
        }
    }
    if let Some(root) = env::var_os("LOCALAPPDATA").filter(|value| !value.is_empty()) {
        let root = PathBuf::from(root);
        directories.push(root.join(&definition.name));
        directories.push(root.join("Programs").join(&definition.name));
    }
    directories
}

fn executable_search_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(base_dirs) = directories::BaseDirs::new() {
        let state_root = base_dirs
            .state_dir()
            .unwrap_or_else(|| base_dirs.home_dir());
        directories.push(state_root.join("nix/profiles/lunchbox/bin"));
    }
    if let Some(path) = env::var_os("PATH") {
        directories.extend(env::split_paths(&path));
    }
    directories
}

fn find_executable_in_paths(name: &str, paths: &[PathBuf]) -> Option<PathBuf> {
    for directory in paths {
        let path = directory.join(name);
        if executable_file(&path) {
            return Some(path);
        }
        #[cfg(windows)]
        {
            if Path::new(name).extension().is_none() {
                for extension in windows_executable_extensions() {
                    let path = directory.join(format!("{name}{extension}"));
                    if executable_file(&path) {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

fn executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(windows)]
fn windows_executable_extensions() -> Vec<String> {
    env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned())
        .split(';')
        .filter(|extension| !extension.is_empty())
        .map(|extension| extension.to_ascii_lowercase())
        .collect()
}

fn executable_names(name: &str, host: HostPlatform) -> Vec<String> {
    let names: &[&str] = match name.to_ascii_lowercase().as_str() {
        "dosbox staging" => &["dosbox", "dosbox-staging"],
        "dosbox-x" => &["dosbox-x", "DOSBox-X"],
        "scummvm" => &["scummvm", "ScummVM"],
        "86box" => &["86Box", "86box"],
        "pcbox" => &["PCBox", "pcbox"],
        _ => &[],
    };
    let mut result = if names.is_empty() {
        vec![name.to_owned(), name.to_ascii_lowercase()]
    } else {
        names.iter().map(|name| (*name).to_owned()).collect()
    };
    if host == HostPlatform::Windows {
        let windows = result
            .iter()
            .filter(|name| Path::new(name).extension().is_none())
            .map(|name| format!("{name}.exe"))
            .collect::<Vec<_>>();
        result.extend(windows);
    }
    result.sort();
    result.dedup();
    result
}

fn discover_macos_application(
    definition: &EmulatorDefinition,
    host: HostPlatform,
) -> Option<EmulatorExecutable> {
    if host != HostPlatform::MacOs {
        return None;
    }
    let roots = [PathBuf::from("/Applications"), {
        directories::BaseDirs::new()?
            .home_dir()
            .join("Applications")
    }];
    for root in roots {
        let bundle = root.join(format!("{}.app", definition.name));
        let macos = bundle.join("Contents/MacOS");
        for name in executable_names(&definition.name, host) {
            let executable = macos.join(name);
            if executable_file(&executable) {
                return Some(EmulatorExecutable::Native(executable));
            }
        }
        if let Ok(entries) = fs::read_dir(&macos) {
            for entry in entries.flatten() {
                if executable_file(&entry.path()) {
                    return Some(EmulatorExecutable::Native(entry.path()));
                }
            }
        }
    }
    None
}

fn installed_flatpak_apps(host: HostPlatform) -> BTreeSet<String> {
    if host != HostPlatform::Linux {
        return BTreeSet::new();
    }
    let paths = executable_search_directories();
    let Some(flatpak) = find_executable_in_paths("flatpak", &paths) else {
        return BTreeSet::new();
    };
    Command::new(flatpak)
        .args(["list", "--app", "--columns=application"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn build_plan_for_choice(
    prepared: &PreparedInstall,
    kind: PreparedLaunchKind,
    emulator: EmulatorChoice,
) -> Result<LaunchPlan> {
    let mut cleanup_paths = Vec::new();
    let arguments = match kind {
        PreparedLaunchKind::Dosbox {
            config_path,
            shared_options_path,
            copy_mt32_roms,
        } => {
            if copy_mt32_roms {
                cleanup_paths = prepare_mt32_exception_files(&prepared.install_root)?;
            }
            let mut arguments = vec![
                OsString::from("-conf"),
                path_argument(&config_path, &emulator),
            ];
            if let Some(options) = shared_options_path {
                arguments.push(OsString::from("-conf"));
                arguments.push(path_argument(&options, &emulator));
            }
            arguments
        }
        PreparedLaunchKind::ScummVm(plan) => {
            let config_path = ensure_scummvm_config(&prepared.install_root, &plan.config_path)?;
            let game_path = safe_install_path(&prepared.install_root, &plan.game_path)?;
            if !game_path.is_dir() {
                bail!(
                    "ScummVM game directory {} does not exist in the prepared install",
                    game_path.display()
                );
            }
            let mut arguments = vec![
                OsString::from("--config"),
                path_argument(&config_path, &emulator),
            ];
            arguments.extend(plan.extra_args.into_iter().map(OsString::from));
            arguments.push(OsString::from("-p"));
            arguments.push(path_argument(&game_path, &emulator));
            arguments.push(OsString::from(plan.game_id));
            arguments
        }
        PreparedLaunchKind::Win9xDosboxX {
            config_path,
            shared_options_path,
        } => vec![
            OsString::from("-conf"),
            path_argument(&config_path, &emulator),
            OsString::from("-conf"),
            path_argument(&shared_options_path, &emulator),
            OsString::from("-nomenu"),
            OsString::from("-noconsole"),
        ],
        PreparedLaunchKind::EightySixBox(plan) => {
            let vm_root = prepare_86box_vm(prepared, &plan)?;
            vec![OsString::from("-P"), path_argument(&vm_root, &emulator)]
        }
        PreparedLaunchKind::PcBox(plan) => {
            let native_pcbox = emulator.name.eq_ignore_ascii_case("PCBox");
            let (vm_root, config_path) = prepare_pcbox_vm(prepared, &plan, native_pcbox)?;
            if native_pcbox {
                if matches!(emulator.executable, EmulatorExecutable::Flatpak { .. }) {
                    bail!("PCBox prepared installs currently require a native PCBox executable")
                }
                vec![OsString::from("-c"), config_path.into_os_string()]
            } else {
                vec![OsString::from("-P"), path_argument(&vm_root, &emulator)]
            }
        }
    };

    let (program, mut prefix_arguments) =
        command_prefix(&emulator.executable, &prepared.install_root)?;
    prefix_arguments.extend(arguments);
    Ok(LaunchPlan {
        emulator_name: emulator.name,
        program,
        arguments: prefix_arguments,
        current_directory: prepared.install_root.clone(),
        cleanup_paths,
    })
}

fn command_prefix(
    executable: &EmulatorExecutable,
    install_root: &Path,
) -> Result<(PathBuf, Vec<OsString>)> {
    match executable {
        EmulatorExecutable::Native(path) => Ok((path.clone(), Vec::new())),
        EmulatorExecutable::Flatpak { command, app_id } => {
            let mount = flatpak_mount_point(install_root)?;
            Ok((
                command.clone(),
                vec![
                    OsString::from("run"),
                    OsString::from(format!(
                        "--filesystem={}",
                        map_path_for_flatpak(&mount).display()
                    )),
                    OsString::from(app_id),
                ],
            ))
        }
    }
}

fn path_argument(path: &Path, emulator: &EmulatorChoice) -> OsString {
    match emulator.executable {
        EmulatorExecutable::Flatpak { .. } => map_path_for_flatpak(path).into_os_string(),
        EmulatorExecutable::Native(_) => path.as_os_str().to_owned(),
    }
}

fn flatpak_mount_point(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        canonical
            .parent()
            .map(Path::to_path_buf)
            .context("Flatpak launch path has no parent directory")
    }
}

fn map_path_for_flatpak(path: &Path) -> PathBuf {
    if !Path::new("/var/home").is_dir() {
        return path.to_path_buf();
    }
    let Some(base_dirs) = directories::BaseDirs::new() else {
        return path.to_path_buf();
    };
    let home = base_dirs.home_dir();
    if home.starts_with("/var/home") {
        return path.to_path_buf();
    }
    let Ok(relative) = path.strip_prefix(home) else {
        return path.to_path_buf();
    };
    let Some(user) = home.file_name() else {
        return path.to_path_buf();
    };
    PathBuf::from("/var/home").join(user).join(relative)
}

fn detect_win9x_launcher_kind(metadata_dir: &Path) -> Result<Win9xLauncherKind> {
    let mut launchers = fs::read_dir(metadata_dir)
        .with_context(|| {
            format!(
                "reading Win9x metadata directory {}",
                metadata_dir.display()
            )
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("bat"))
                && !path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.eq_ignore_ascii_case("install.bat"))
        })
        .collect::<Vec<_>>();
    launchers.sort();
    let Some(launcher) = launchers.first() else {
        return Ok(Win9xLauncherKind::DosboxX);
    };
    let contents = fs::read_to_string(launcher)
        .with_context(|| format!("reading eXoWin9x launcher {}", launcher.display()))?;
    let lower = contents.replace("\r\n", "\n").to_ascii_lowercase();
    if lower.contains("9xlaunchpcbox") {
        return Ok(Win9xLauncherKind::PcBox(PcBoxPlan {
            config_name: "Play.cfg",
            parent_vhd_name: "W98-P.vhd",
            child_vhd_name: "W98-C.vhd",
        }));
    }
    let plan = if lower.contains("9xlaunch86boxnethost") {
        Some(EightySixBoxPlan {
            config_name: "Host.cfg",
            parent_vhd_name: "W98-NetHost.vhd",
            child_vhd_name: "W98-Host.vhd",
        })
    } else if lower.contains("9xlaunch86boxnetjoin") {
        Some(EightySixBoxPlan {
            config_name: "Join.cfg",
            parent_vhd_name: "W98-NetJoin.vhd",
            child_vhd_name: "W98-Join.vhd",
        })
    } else if lower.contains("9xlaunch86boxme") {
        Some(EightySixBoxPlan {
            config_name: "Play.cfg",
            parent_vhd_name: "ME-P.vhd",
            child_vhd_name: "ME-C.vhd",
        })
    } else if lower.contains("9xlaunch86box") {
        Some(EightySixBoxPlan {
            config_name: "Play.cfg",
            parent_vhd_name: "W98-P.vhd",
            child_vhd_name: "W98-C.vhd",
        })
    } else {
        None
    };
    Ok(plan
        .map(Win9xLauncherKind::EightySixBox)
        .unwrap_or(Win9xLauncherKind::DosboxX))
}

fn prepare_86box_vm(prepared: &PreparedInstall, plan: &EightySixBoxPlan) -> Result<PathBuf> {
    let vm_root = prepared.install_root.join("emulators/86Box98");
    prepare_vm_files(
        prepared,
        &vm_root,
        "86box.cfg",
        plan.config_name,
        plan.parent_vhd_name,
        plan.child_vhd_name,
    )?;
    Ok(vm_root)
}

fn prepare_pcbox_vm(
    prepared: &PreparedInstall,
    plan: &PcBoxPlan,
    native_pcbox: bool,
) -> Result<(PathBuf, PathBuf)> {
    let vm_root = prepared.install_root.join("emulators/PCBox");
    let config_name = if native_pcbox {
        "play.cfg"
    } else {
        "86box.cfg"
    };
    let config_path = prepare_vm_files(
        prepared,
        &vm_root,
        config_name,
        plan.config_name,
        plan.parent_vhd_name,
        plan.child_vhd_name,
    )?;
    Ok((vm_root, config_path))
}

fn prepare_vm_files(
    prepared: &PreparedInstall,
    vm_root: &Path,
    output_config_name: &str,
    source_config_name: &str,
    parent_vhd_name: &str,
    child_vhd_name: &str,
) -> Result<PathBuf> {
    fs::create_dir_all(vm_root)
        .with_context(|| format!("creating virtual machine directory {}", vm_root.display()))?;
    let parent_disk = vm_root.join("parent").join(parent_vhd_name);
    if !parent_disk.is_file() {
        bail!(
            "prepared eXoWin9x install is missing parent disk {}",
            parent_disk.display()
        );
    }
    let child_disk = vm_root.join(child_vhd_name);
    if !child_disk.exists() {
        reflink_copy::reflink(&parent_disk, &child_disk)
            .or_else(|_| fs::copy(&parent_disk, &child_disk).map(|_| ()))
            .with_context(|| {
                format!(
                    "creating writable virtual machine disk {} from {}",
                    child_disk.display(),
                    parent_disk.display()
                )
            })?;
    }
    let metadata_dir = prepared
        .launch_config_path
        .parent()
        .context("prepared Win9x config has no parent directory")?;
    let source_config = metadata_dir.join(source_config_name);
    if !source_config.is_file() {
        bail!(
            "prepared eXoWin9x install is missing VM config {}",
            source_config.display()
        );
    }
    let output_config = vm_root.join(output_config_name);
    if !output_config.exists() {
        fs::copy(&source_config, &output_config).with_context(|| {
            format!(
                "installing VM config {} from {}",
                output_config.display(),
                source_config.display()
            )
        })?;
    }
    Ok(output_config)
}

fn prepare_mt32_exception_files(install_root: &Path) -> Result<Vec<PathBuf>> {
    let source = install_root.join("mt32");
    if !source.is_dir() {
        bail!(
            "this eXo exception launcher expects MT-32 ROMs in {}",
            source.display()
        );
    }
    let mut cleanup = Vec::new();
    for entry in fs::read_dir(&source)
        .with_context(|| format!("reading MT-32 directory {}", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("rom"))
        {
            continue;
        }
        let target = install_root.join(entry.file_name());
        if target.exists() {
            continue;
        }
        fs::copy(&path, &target).with_context(|| {
            format!(
                "copying MT-32 ROM {} to {}",
                path.display(),
                target.display()
            )
        })?;
        cleanup.push(target);
    }
    Ok(cleanup)
}

fn ensure_scummvm_config(install_root: &Path, relative: &Path) -> Result<PathBuf> {
    let path = safe_install_path(install_root, relative)?;
    if path.exists() {
        return Ok(path);
    }
    let parent = path
        .parent()
        .context("ScummVM configuration path has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating ScummVM directory {}", parent.display()))?;
    let staging = parent.join(format!(".scummvm-config-{}", uuid::Uuid::new_v4()));
    fs::write(&staging, DEFAULT_SCUMMVM_CONFIG)
        .with_context(|| format!("writing ScummVM configuration {}", staging.display()))?;
    fs::rename(&staging, &path)
        .with_context(|| format!("publishing ScummVM configuration {}", path.display()))?;
    Ok(path)
}

fn safe_install_path(install_root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.as_os_str().is_empty() || relative.is_absolute() {
        bail!("prepared launch path must be a non-empty relative path")
    }
    let mut safe = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => safe.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!(
                    "prepared launch path escapes the install root: {}",
                    relative.display()
                )
            }
        }
    }
    if safe.as_os_str().is_empty() {
        bail!("prepared launch path does not name a file or directory")
    }
    Ok(install_root.join(safe))
}

pub fn parse_dosbox_exception_script(contents: &str) -> Result<DosboxExceptionPlan> {
    let normalized = contents.replace("\r\n", "\n").replace('\r', "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let branch = extract_dosbox_branch_lines(&lines);
    let marker = r#".\emulators\dosbox\%dosbox%"#;
    if !branch
        .iter()
        .any(|line| line.to_ascii_lowercase().contains(marker))
    {
        bail!("exception launcher does not contain a supported DOSBox branch")
    }
    let mut plan = DosboxExceptionPlan {
        copy_mt32_roms: false,
    };
    for original in branch {
        let line = original.trim();
        let lower = line.to_ascii_lowercase();
        if lower.is_empty()
            || lower == "echo off"
            || lower == "cls"
            || lower.starts_with("echo.")
            || lower.starts_with("echo ")
            || lower.starts_with("cd ")
            || lower.starts_with("goto ")
            || lower.starts_with(':')
            || lower.contains("setconsole.exe")
            || lower == "del stdout.txt"
            || lower == "del stderr.txt"
            || (lower.starts_with("if exist ") && lower.contains(" del "))
        {
            continue;
        }
        if lower == r#"copy .\mt32\*.rom .\"# {
            plan.copy_mt32_roms = true;
            continue;
        }
        if lower == "del *.rom" {
            continue;
        }
        if lower.contains(marker) {
            if !lower.trim_start_matches('"').starts_with(marker) {
                bail!("unsupported DOSBox wrapper command: {line}")
            }
            continue;
        }
        if lower.starts_with("start ") || lower.starts_with("taskkill ") {
            bail!("exception launcher requires unsupported helper process management: {line}")
        }
        bail!("unsupported command in the DOSBox exception path: {line}")
    }
    Ok(plan)
}

pub fn parse_linux_dosbox_exception_script(contents: &str) -> Result<LinuxDosboxExceptionPlan> {
    let normalized = contents.replace("\r\n", "\n").replace('\r', "\n");
    let mut names = Vec::new();
    for line in normalized.lines() {
        if !line.to_ascii_lowercase().contains("options_linux.conf") {
            continue;
        }
        for name in extract_linux_config_names(line) {
            if !name.eq_ignore_ascii_case("options_linux.conf")
                && !names
                    .iter()
                    .any(|existing: &String| existing.eq_ignore_ascii_case(&name))
            {
                names.push(name);
            }
        }
    }
    if names.is_empty() {
        bail!("exception launcher does not contain a supported Linux DOSBox branch")
    }
    let launch_config_name = names
        .iter()
        .find(|name| name.eq_ignore_ascii_case("dosbox_linux.conf"))
        .cloned()
        .unwrap_or_else(|| names[0].clone());
    if Path::new(&launch_config_name).components().count() != 1 {
        bail!("Linux DOSBox exception config is not a file name")
    }
    Ok(LinuxDosboxExceptionPlan { launch_config_name })
}

pub fn parse_linux_scummvm_exception_script(contents: &str) -> Result<LinuxScummvmExceptionPlan> {
    let normalized = contents.replace("\r\n", "\n").replace('\r', "\n");
    for line in normalized.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if trimmed.is_empty() || !lower.contains("scummvm") || !lower.contains("-p") {
            continue;
        }
        let tokens = trimmed
            .split_whitespace()
            .map(normalize_shellish_token)
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>();
        if tokens.is_empty() {
            continue;
        }
        let arguments = if tokens.first().map(String::as_str) == Some("flatpak")
            && tokens.get(1).map(String::as_str) == Some("run")
        {
            if tokens.len() < 4 || !tokens[2].to_ascii_lowercase().contains("scummvm") {
                continue;
            }
            &tokens[3..]
        } else {
            if !tokens[0].to_ascii_lowercase().contains("scummvm") {
                continue;
            }
            &tokens[1..]
        };
        let mut config_path = None;
        let mut game_path = None;
        let mut extra_args = Vec::new();
        let mut positionals = Vec::new();
        let mut index = 0;
        while index < arguments.len() {
            let token = &arguments[index];
            if let Some(value) = token.strip_prefix("--config=") {
                config_path = Some(value.to_owned());
                index += 1;
            } else if token == "--config" {
                config_path = Some(
                    arguments
                        .get(index + 1)
                        .context("ScummVM exception launcher is missing the config path")?
                        .clone(),
                );
                index += 2;
            } else if let Some(value) = token.strip_prefix("-p") {
                if value.is_empty() {
                    game_path = Some(
                        arguments
                            .get(index + 1)
                            .context("ScummVM exception launcher is missing the game path")?
                            .clone(),
                    );
                    index += 2;
                } else {
                    game_path = Some(value.to_owned());
                    index += 1;
                }
            } else {
                if token.starts_with('-') {
                    extra_args.push(token.clone());
                } else {
                    positionals.push(token.clone());
                }
                index += 1;
            }
        }
        let config_path =
            PathBuf::from(config_path.context("ScummVM exception launcher is missing --config")?);
        let game_path =
            PathBuf::from(game_path.context("ScummVM exception launcher is missing -p")?);
        validate_relative_launch_path(&config_path)?;
        validate_relative_launch_path(&game_path)?;
        let game_id = positionals
            .pop()
            .context("ScummVM exception launcher is missing the game id")?;
        return Ok(LinuxScummvmExceptionPlan {
            config_path,
            game_path,
            game_id,
            extra_args,
        });
    }
    bail!("exception launcher does not contain a supported Linux ScummVM branch")
}

fn validate_relative_launch_path(path: &Path) -> Result<()> {
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!(
            "exception launcher path escapes the prepared install: {}",
            path.display()
        )
    }
    Ok(())
}

fn normalize_shellish_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .replace("\\!", "!")
}

fn extract_linux_config_names(line: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut search_start = 0;
    const SUFFIX: &str = "_linux.conf";
    while let Some(relative_end) = line[search_start..].find(SUFFIX) {
        let end = search_start + relative_end + SUFFIX.len();
        let mut start = search_start + relative_end;
        while start > 0 {
            let character = line.as_bytes()[start - 1] as char;
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
                start -= 1;
            } else {
                break;
            }
        }
        if start < end {
            names.push(line[start..end].to_owned());
        }
        search_start = end;
    }
    names
}

fn extract_dosbox_branch_lines<'a>(lines: &'a [&'a str]) -> Vec<&'a str> {
    let start = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case(":dosbox"))
        .map_or(0, |index| index + 1);
    let mut branch = Vec::new();
    for line in &lines[start..] {
        let trimmed = line.trim();
        if !branch.is_empty() && trimmed.starts_with(':') {
            break;
        }
        branch.push(*line);
    }
    branch
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn prepared(root: &Path, collection: ExoCollection, config: &str) -> PreparedInstall {
        let launch_config_path = root.join(config);
        fs::create_dir_all(launch_config_path.parent().unwrap()).unwrap();
        fs::write(&launch_config_path, b"config").unwrap();
        PreparedInstall {
            collection,
            install_root: root.to_path_buf(),
            launch_config_path,
            shortname: "TEST".to_owned(),
            reused: true,
        }
    }

    #[test]
    fn parses_supported_dosbox_and_scummvm_exceptions_without_executing_scripts() {
        let dosbox = parse_dosbox_exception_script(
            r#"@echo off
:dosbox
copy .\mt32\*.rom .\
.\emulators\dosbox\%dosbox% -conf .\game.conf
del *.rom
:end
"#,
        )
        .unwrap();
        assert!(dosbox.copy_mt32_roms);

        let scummvm = parse_linux_scummvm_exception_script(
            r#"flatpak run org.scummvm.ScummVM --config=./emulators/scummvm/scummvm.ini -F -g3x -p./eXoDOS/120Deg sci-fanmade"#,
        )
        .unwrap();
        assert_eq!(
            scummvm.config_path,
            Path::new("./emulators/scummvm/scummvm.ini")
        );
        assert_eq!(scummvm.game_path, Path::new("./eXoDOS/120Deg"));
        assert_eq!(scummvm.game_id, "sci-fanmade");
        assert_eq!(scummvm.extra_args, ["-F", "-g3x"]);
    }

    #[test]
    fn rejects_exception_paths_that_escape_the_prepared_install() {
        let error = parse_linux_scummvm_exception_script(
            "scummvm --config=../../outside.ini -p./game target",
        )
        .unwrap_err();
        assert!(error.to_string().contains("escapes"));
    }

    #[test]
    fn classifies_all_win9x_launcher_families_exactly() {
        let temp = TempDir::new().unwrap();
        let cases = [
            (".\\util\\9xlaunch.bat", "dosbox"),
            (".\\util\\9xlaunch86Box.bat", "86box"),
            (".\\util\\9xlaunch86BoxNetHost.bat", "host"),
            (".\\util\\9xlaunch86BoxNetJoin.bat", "join"),
            (".\\util\\9xlaunch86BoxME.bat", "me"),
            (".\\util\\9xlaunchPCBox.bat", "pcbox"),
        ];
        for (index, (contents, expected)) in cases.into_iter().enumerate() {
            let directory = temp.path().join(index.to_string());
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join("Game.bat"), contents).unwrap();
            let actual = detect_win9x_launcher_kind(&directory).unwrap();
            match (expected, actual) {
                ("dosbox", Win9xLauncherKind::DosboxX)
                | (
                    "86box",
                    Win9xLauncherKind::EightySixBox(EightySixBoxPlan {
                        config_name: "Play.cfg",
                        parent_vhd_name: "W98-P.vhd",
                        child_vhd_name: "W98-C.vhd",
                    }),
                )
                | (
                    "host",
                    Win9xLauncherKind::EightySixBox(EightySixBoxPlan {
                        config_name: "Host.cfg",
                        parent_vhd_name: "W98-NetHost.vhd",
                        child_vhd_name: "W98-Host.vhd",
                    }),
                )
                | (
                    "join",
                    Win9xLauncherKind::EightySixBox(EightySixBoxPlan {
                        config_name: "Join.cfg",
                        parent_vhd_name: "W98-NetJoin.vhd",
                        child_vhd_name: "W98-Join.vhd",
                    }),
                )
                | (
                    "me",
                    Win9xLauncherKind::EightySixBox(EightySixBoxPlan {
                        config_name: "Play.cfg",
                        parent_vhd_name: "ME-P.vhd",
                        child_vhd_name: "ME-C.vhd",
                    }),
                )
                | (
                    "pcbox",
                    Win9xLauncherKind::PcBox(PcBoxPlan {
                        config_name: "Play.cfg",
                        parent_vhd_name: "W98-P.vhd",
                        child_vhd_name: "W98-C.vhd",
                    }),
                ) => {}
                (_, other) => panic!("unexpected launcher classification: {other:?}"),
            }
        }
    }

    #[test]
    fn creates_persistent_vm_child_disk_without_hardlinking_parent() {
        let temp = TempDir::new().unwrap();
        let prepared = prepared(
            temp.path(),
            ExoCollection::Win9x,
            "eXoWin9x/!win9x/1996/Test/Play.cfg",
        );
        let vm_root = temp.path().join("emulators/86Box98");
        fs::create_dir_all(vm_root.join("parent")).unwrap();
        fs::write(vm_root.join("parent/W98-P.vhd"), b"parent").unwrap();
        let plan = EightySixBoxPlan {
            config_name: "Play.cfg",
            parent_vhd_name: "W98-P.vhd",
            child_vhd_name: "W98-C.vhd",
        };
        let first = prepare_86box_vm(&prepared, &plan).unwrap();
        fs::write(first.join("W98-C.vhd"), b"changed child").unwrap();
        let second = prepare_86box_vm(&prepared, &plan).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            fs::read(second.join("W98-C.vhd")).unwrap(),
            b"changed child"
        );
        assert_eq!(
            fs::read(second.join("parent/W98-P.vhd")).unwrap(),
            b"parent"
        );
        assert_eq!(fs::read(second.join("86box.cfg")).unwrap(), b"config");
    }

    #[test]
    fn canonical_catalog_drives_host_specific_emulator_metadata() {
        let temp = TempDir::new().unwrap();
        let database = temp.path().join("catalog.db");
        let connection = Connection::open(&database).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE emulators(id TEXT PRIMARY KEY, name TEXT);
                 CREATE TABLE emulator_host_systems(emulator_id TEXT, host_system_slug TEXT);
                 CREATE TABLE emulator_packages(emulator_id TEXT, manager TEXT, package_id TEXT);
                 INSERT INTO emulators VALUES('dosbox-x-id','DOSBox-X');
                 INSERT INTO emulators VALUES('wrong-host','DOSBox Staging');
                 INSERT INTO emulator_host_systems VALUES('dosbox-x-id','linux');
                 INSERT INTO emulator_host_systems VALUES('wrong-host','windows');
                 INSERT INTO emulator_packages VALUES('dosbox-x-id','flatpak','com.dosbox_x.DOSBox-X');",
            )
            .unwrap();
        let definitions = load_emulator_definitions(
            &database,
            HostPlatform::Linux,
            &["DOSBox Staging", "DOSBox-X"],
        )
        .unwrap();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].id, "dosbox-x-id");
        assert_eq!(
            definitions[0].packages["flatpak"],
            ["com.dosbox_x.DOSBox-X"]
        );
    }

    #[test]
    fn flatpak_plan_passes_paths_as_arguments_without_a_shell() {
        let temp = TempDir::new().unwrap();
        let prepared = prepared(
            temp.path(),
            ExoCollection::Dos,
            "eXoDOS/!dos/TEST/dosbox_linux.conf",
        );
        let options = temp.path().join("emulators/dosbox/options_linux.conf");
        fs::create_dir_all(options.parent().unwrap()).unwrap();
        fs::write(&options, b"options").unwrap();
        let kind = classify_prepared_install(&prepared).unwrap();
        let choice = EmulatorChoice {
            id: "dosbox-x".to_owned(),
            name: "DOSBox-X".to_owned(),
            executable: EmulatorExecutable::Flatpak {
                command: PathBuf::from("/usr/bin/flatpak"),
                app_id: "com.dosbox_x.DOSBox-X".to_owned(),
            },
        };
        let plan = build_plan_for_choice(&prepared, kind, choice).unwrap();
        assert_eq!(plan.program, Path::new("/usr/bin/flatpak"));
        assert_eq!(plan.arguments[0], "run");
        assert!(
            plan.arguments[1]
                .to_string_lossy()
                .starts_with("--filesystem=")
        );
        assert_eq!(plan.arguments[2], "com.dosbox_x.DOSBox-X");
        assert_eq!(plan.arguments[3], "-conf");
        assert_eq!(plan.arguments[4], prepared.launch_config_path.as_os_str());
    }
}
