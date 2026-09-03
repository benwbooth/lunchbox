use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::params_from_iter;

use crate::exo_install::{ExoCollection, PreparedInstall};
use crate::platform_process::{host_command, is_flatpak};

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
struct PlatformEmulatorDefinition {
    emulator: EmulatorDefinition,
    cores: Vec<String>,
    recommended: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmulatorExecutable {
    Native(PathBuf),
    Flatpak {
        command: PathBuf,
        app_id: String,
    },
    Wine {
        command: PathBuf,
        executable: PathBuf,
        prefix: PathBuf,
    },
}

impl EmulatorExecutable {
    pub fn summary(&self) -> String {
        match self {
            Self::Native(path) => path.display().to_string(),
            Self::Flatpak { app_id, .. } => format!("Flatpak · {app_id}"),
            Self::Wine { executable, .. } => format!("Wine · {}", executable.display()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmulatorChoice {
    pub id: String,
    pub name: String,
    pub executable: EmulatorExecutable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmulatorRuntimeKind {
    Standalone,
    RetroArch,
}

impl EmulatorRuntimeKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::RetroArch => "retroarch",
        }
    }

    fn sort_key(self) -> u8 {
        match self {
            Self::RetroArch => 0,
            Self::Standalone => 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RomEmulatorOption {
    pub emulator_id: String,
    pub emulator_name: String,
    pub runtime_kind: EmulatorRuntimeKind,
    pub core_name: String,
    pub executable: EmulatorExecutable,
    core_path: Option<PathBuf>,
    recommended: bool,
}

impl RomEmulatorOption {
    pub fn label(&self) -> String {
        match self.runtime_kind {
            EmulatorRuntimeKind::Standalone => self.emulator_name.clone(),
            EmulatorRuntimeKind::RetroArch => {
                format!("RetroArch · {} ({})", self.emulator_name, self.core_name)
            }
        }
    }

    pub fn summary(&self) -> String {
        match self.runtime_kind {
            EmulatorRuntimeKind::Standalone => self.executable.summary(),
            EmulatorRuntimeKind::RetroArch => format!(
                "{} · core {}",
                self.executable.summary(),
                self.core_path
                    .as_deref()
                    .map(Path::display)
                    .map(|display| display.to_string())
                    .unwrap_or_else(|| self.core_name.clone())
            ),
        }
    }

    fn matches_preference(&self, preference: &crate::settings::EmulatorPreference) -> bool {
        self.emulator_id == preference.emulator_id
            && self.runtime_kind.key() == preference.runtime_kind
            && self.core_name == preference.core_name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RomLaunchAvailability {
    pub options: Vec<RomEmulatorOption>,
    pub selected_index: Option<usize>,
    pub preference_scope: String,
    pub requirement: String,
    pub detail: String,
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
    pub environment: Vec<(OsString, OsString)>,
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

const MAX_LAUNCH_ARGUMENT_TEXT: usize = 8 * 1024;
const MAX_LAUNCH_ARGUMENTS: usize = 256;
const TEMPLATE_SENTINEL_PREFIX: &str = "__LUNCHBOX_ARG_";
const LAUNCH_TEMPLATE_PLACEHOLDERS: &[&str] = &[
    "file",
    "core",
    "mame_rompath",
    "mame_romset",
    "hypseus_game",
    "hypseus_framefile",
    "hypseus_support_root",
    "hypseus_romdir",
    "altirra_media_switch",
    "config",
    "shared_config",
    "vm_root",
    "game_path",
    "game_id",
];

#[derive(Clone, Debug, Eq, PartialEq)]
enum LaunchTemplateValue {
    Literal(OsString),
    Path(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchCommandPreview {
    pub runtime: String,
    pub arguments: Vec<String>,
    pub uses_custom_template: bool,
    pub extra_arguments_ignored: bool,
}

impl LaunchCommandPreview {
    pub fn summary(&self) -> String {
        let argument_count = self.arguments.len();
        if self.extra_arguments_ignored {
            format!(
                "Custom template · {argument_count} argv token{} · extra arguments are valid but ignored",
                if argument_count == 1 { "" } else { "s" }
            )
        } else if self.uses_custom_template {
            format!(
                "Custom template · {argument_count} direct argv token{}",
                if argument_count == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "Built-in template · {argument_count} direct argv token{}",
                if argument_count == 1 { "" } else { "s" }
            )
        }
    }
}

pub fn validate_launch_extra_arguments(arguments: &str) -> Result<()> {
    parse_portable_arguments(arguments).map(|_| ())
}

pub fn validate_launch_template(template: &str) -> Result<()> {
    if template.len() > MAX_LAUNCH_ARGUMENT_TEXT {
        bail!("launch command templates must be at most {MAX_LAUNCH_ARGUMENT_TEXT} bytes");
    }
    if template.contains(TEMPLATE_SENTINEL_PREFIX) {
        bail!("launch command template contains a reserved internal token");
    }
    let placeholders = template_placeholders(template)?;
    for placeholder in placeholders {
        if !LAUNCH_TEMPLATE_PLACEHOLDERS.contains(&placeholder.as_str()) {
            bail!("launch command template uses unknown placeholder %{{{placeholder}}}");
        }
    }
    parse_portable_arguments(&replace_template_placeholders_with_tokens(template)?)?;
    Ok(())
}

pub fn parse_portable_arguments(arguments: &str) -> Result<Vec<String>> {
    if arguments.len() > MAX_LAUNCH_ARGUMENT_TEXT {
        bail!("launch arguments must be at most {MAX_LAUNCH_ARGUMENT_TEXT} bytes");
    }
    let mut parsed = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut token_started = false;
    let mut characters = arguments.chars().peekable();
    while let Some(character) = characters.next() {
        match quote {
            Some('\'') => {
                if character == '\'' {
                    quote = None;
                } else {
                    current.push(character);
                }
            }
            Some('"') => match character {
                '"' => quote = None,
                '\\' if matches!(characters.peek(), Some('"')) => {
                    current.push(characters.next().expect("peeked argument character"));
                }
                _ => current.push(character),
            },
            Some(_) => unreachable!("portable argument parser has only two quote modes"),
            None => match character {
                '\'' | '"' => {
                    quote = Some(character);
                    token_started = true;
                }
                '\\' if matches!(
                    characters.peek(),
                    Some(next) if next.is_whitespace() || matches!(next, '\'' | '"')
                ) =>
                {
                    current.push(characters.next().expect("peeked argument character"));
                    token_started = true;
                }
                value if value.is_whitespace() => {
                    if token_started {
                        parsed.push(std::mem::take(&mut current));
                        token_started = false;
                    }
                }
                _ => {
                    current.push(character);
                    token_started = true;
                }
            },
        }
    }
    if let Some(delimiter) = quote {
        bail!("launch arguments contain an unterminated {delimiter} quote");
    }
    if token_started {
        parsed.push(current);
    }
    if parsed.len() > MAX_LAUNCH_ARGUMENTS {
        bail!("launch commands may contain at most {MAX_LAUNCH_ARGUMENTS} arguments");
    }
    Ok(parsed)
}

pub fn default_rom_launch_template(option: &RomEmulatorOption, platform: &str) -> String {
    default_rom_launch_template_for(&option.emulator_name, option.runtime_kind, platform)
}

pub fn default_rom_launch_template_for(
    emulator_name: &str,
    runtime_kind: EmulatorRuntimeKind,
    platform: &str,
) -> String {
    if runtime_kind == EmulatorRuntimeKind::RetroArch {
        return "--verbose -L %{core} %f".to_owned();
    }
    if emulator_name.eq_ignore_ascii_case("MAME") && is_arcade_family_platform(platform) {
        return "-rompath %{mame_rompath} %{mame_romset}".to_owned();
    }
    if emulator_name.eq_ignore_ascii_case("Hypseus Singe") && is_arcade_family_platform(platform) {
        return "%{hypseus_game} vldp -fullscreen -framefile %{hypseus_framefile} -homedir %{hypseus_support_root} -datadir %{hypseus_support_root} -romdir %{hypseus_romdir}".to_owned();
    }
    if emulator_name.eq_ignore_ascii_case("Altirra") {
        return "%{altirra_media_switch} %f".to_owned();
    }
    "%f".to_owned()
}

pub fn default_rom_extra_argument_insert_index(
    emulator_name: &str,
    runtime_kind: EmulatorRuntimeKind,
    platform: &str,
) -> usize {
    if runtime_kind == EmulatorRuntimeKind::RetroArch {
        3
    } else if emulator_name.eq_ignore_ascii_case("Hypseus Singe")
        && is_arcade_family_platform(platform)
    {
        3
    } else {
        0
    }
}

pub fn default_prepared_launch_template(
    prepared: &PreparedInstall,
    emulator_name: &str,
) -> Result<String> {
    let kind = classify_prepared_install(prepared)?;
    Ok(prepared_launch_template(&kind, emulator_name))
}

fn prepared_launch_template(kind: &PreparedLaunchKind, emulator_name: &str) -> String {
    match kind {
        PreparedLaunchKind::Dosbox {
            shared_options_path,
            ..
        } => {
            if shared_options_path.is_some() {
                "-conf %{config} -conf %{shared_config}".to_owned()
            } else {
                "-conf %{config}".to_owned()
            }
        }
        PreparedLaunchKind::ScummVm(_) => {
            "--config %{config} -p %{game_path} %{game_id}".to_owned()
        }
        PreparedLaunchKind::Win9xDosboxX { .. } => {
            "-conf %{config} -conf %{shared_config} -nomenu -noconsole".to_owned()
        }
        PreparedLaunchKind::EightySixBox(_) => "-P %{vm_root}".to_owned(),
        PreparedLaunchKind::PcBox(_) if emulator_name.eq_ignore_ascii_case("PCBox") => {
            "-c %{config}".to_owned()
        }
        PreparedLaunchKind::PcBox(_) => "-P %{vm_root}".to_owned(),
    }
}

fn template_placeholders(template: &str) -> Result<Vec<String>> {
    let characters = template.chars().collect::<Vec<_>>();
    let mut placeholders = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] != '%' {
            index += 1;
            continue;
        }
        let Some(next) = characters.get(index + 1).copied() else {
            bail!("launch command template ends with a dangling percent sign");
        };
        match next {
            '%' => index += 2,
            'f' => {
                placeholders.push("file".to_owned());
                index += 2;
            }
            '{' => {
                let Some(relative_end) = characters[index + 2..]
                    .iter()
                    .position(|character| *character == '}')
                else {
                    bail!("launch command template has an unterminated placeholder");
                };
                let end = index + 2 + relative_end;
                let name = characters[index + 2..end].iter().collect::<String>();
                if name.is_empty() {
                    bail!("launch command template contains an empty placeholder");
                }
                placeholders.push(name);
                index = end + 1;
            }
            unknown => bail!("launch command template uses unknown placeholder %{unknown}"),
        }
    }
    Ok(placeholders)
}

pub fn launch_template_placeholders(template: &str) -> Result<Vec<String>> {
    validate_launch_template(template)?;
    template_placeholders(template)
}

fn replace_template_placeholders_with_tokens(template: &str) -> Result<String> {
    let characters = template.chars().collect::<Vec<_>>();
    let mut rendered = String::new();
    let mut index = 0;
    let mut placeholder_index = 0;
    while index < characters.len() {
        if characters[index] != '%' {
            rendered.push(characters[index]);
            index += 1;
            continue;
        }
        let next = characters[index + 1];
        match next {
            '%' => {
                rendered.push('%');
                index += 2;
            }
            'f' => {
                rendered.push_str(&format!("{TEMPLATE_SENTINEL_PREFIX}{placeholder_index}__"));
                placeholder_index += 1;
                index += 2;
            }
            '{' => {
                let end = characters[index + 2..]
                    .iter()
                    .position(|character| *character == '}')
                    .map(|relative| index + 2 + relative)
                    .context("launch command template has an unterminated placeholder")?;
                rendered.push_str(&format!("{TEMPLATE_SENTINEL_PREFIX}{placeholder_index}__"));
                placeholder_index += 1;
                index = end + 1;
            }
            _ => unreachable!("template syntax was validated before replacement"),
        }
    }
    Ok(rendered)
}

fn compile_launch_template(
    template: &str,
    values: &BTreeMap<String, LaunchTemplateValue>,
    executable: &EmulatorExecutable,
) -> Result<Vec<OsString>> {
    validate_launch_template(template)?;
    let placeholder_names = template_placeholders(template)?;
    let rendered = replace_template_placeholders_with_tokens(template)?;
    let tokens = parse_portable_arguments(&rendered)?;
    let sentinels = placeholder_names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let value = values.get(&name).cloned().with_context(|| {
                format!("placeholder %{{{name}}} is not available for this launch")
            })?;
            Ok((format!("{TEMPLATE_SENTINEL_PREFIX}{index}__"), value))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut arguments = Vec::with_capacity(tokens.len());
    for token in tokens {
        if let Some((_, value)) = sentinels.iter().find(|(sentinel, _)| sentinel == &token) {
            arguments.push(materialize_template_value(value, executable));
            continue;
        }
        let mut expanded = token;
        for (sentinel, value) in &sentinels {
            if !expanded.contains(sentinel) {
                continue;
            }
            let materialized = materialize_template_value(value, executable);
            let text = materialized.to_str().with_context(|| {
                format!("placeholder embedded in an argument is not valid Unicode: {sentinel}")
            })?;
            expanded = expanded.replace(sentinel, text);
        }
        arguments.push(OsString::from(expanded));
    }
    Ok(arguments)
}

fn materialize_template_value(
    value: &LaunchTemplateValue,
    executable: &EmulatorExecutable,
) -> OsString {
    match value {
        LaunchTemplateValue::Literal(value) => value.clone(),
        LaunchTemplateValue::Path(path) => path_argument_for_executable(path, executable),
    }
}

pub fn default_prepared_extra_argument_insert_index(default_template: &str) -> Result<usize> {
    let arguments = parse_portable_arguments(default_template)?;
    Ok(arguments
        .iter()
        .position(|argument| argument == "-p")
        .unwrap_or(arguments.len()))
}

pub fn effective_launch_preview_values(
    exact_extra_arguments: &str,
    exact_command_template: &str,
    fallback_extra_arguments: &str,
    fallback_command_template: &str,
) -> (String, String) {
    (
        if exact_extra_arguments.trim().is_empty() {
            fallback_extra_arguments.to_owned()
        } else {
            exact_extra_arguments.to_owned()
        },
        if exact_command_template.trim().is_empty() {
            fallback_command_template.to_owned()
        } else {
            exact_command_template.to_owned()
        },
    )
}

pub fn preview_launch_command(
    runtime: &str,
    default_template: &str,
    extra_arguments: &str,
    command_template: &str,
    available_placeholders: &[String],
    extra_insert_index: usize,
) -> Result<LaunchCommandPreview> {
    let parsed_extra_arguments = parse_portable_arguments(extra_arguments)?;
    let command_template = command_template.trim();
    let uses_custom_template = !command_template.is_empty();
    let extra_arguments_ignored = uses_custom_template && !parsed_extra_arguments.is_empty();
    let effective_template = if uses_custom_template {
        command_template
    } else {
        default_template
    };
    let requested_placeholders = launch_template_placeholders(effective_template)?;
    for placeholder in &requested_placeholders {
        if !available_placeholders
            .iter()
            .any(|available| available == placeholder)
        {
            bail!("placeholder %{{{placeholder}}} is unavailable for this exact runtime");
        }
    }

    let values = requested_placeholders
        .iter()
        .map(|placeholder| {
            (
                placeholder.clone(),
                LaunchTemplateValue::Literal(OsString::from(preview_placeholder_value(
                    placeholder,
                ))),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let preview_executable = EmulatorExecutable::Native(PathBuf::from("lunchbox-preview"));
    let mut arguments = compile_launch_template(effective_template, &values, &preview_executable)?
        .into_iter()
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| anyhow!("the launch preview contains non-Unicode data"))
        })
        .collect::<Result<Vec<_>>>()?;

    if !uses_custom_template {
        if extra_insert_index > arguments.len() {
            bail!(
                "the built-in launch profile expected argument position {extra_insert_index}, but produced only {} arguments",
                arguments.len()
            );
        }
        arguments.splice(
            extra_insert_index..extra_insert_index,
            parsed_extra_arguments,
        );
    }

    Ok(LaunchCommandPreview {
        runtime: runtime.trim().to_owned(),
        arguments,
        uses_custom_template,
        extra_arguments_ignored,
    })
}

fn preview_placeholder_value(placeholder: &str) -> &'static str {
    match placeholder {
        "file" => "<selected-file>",
        "core" => "<retroarch-core>",
        "mame_rompath" | "hypseus_romdir" => "<rom-directory>",
        "mame_romset" => "<machine-set>",
        "hypseus_game" => "<game-name>",
        "hypseus_framefile" => "<framefile>",
        "hypseus_support_root" => "<support-directory>",
        "altirra_media_switch" => "<media-switch>",
        "config" => "<game-config>",
        "shared_config" => "<shared-config>",
        "vm_root" => "<virtual-machine>",
        "game_path" => "<game-directory>",
        "game_id" => "<game-id>",
        _ => "<runtime-value>",
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
    let managed_executables = managed_emulator_executables(host, &path_entries);
    let emulator = definitions.iter().find_map(|definition| {
        discover_definition(
            definition,
            host,
            &path_entries,
            &flatpak_apps,
            &managed_executables,
        )
    });
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

pub fn build_prepared_launch_plan_with_customization(
    prepared: &PreparedInstall,
    catalog_database: &Path,
    expected_emulator_id: &str,
    customization: &crate::settings::ResolvedLaunchCustomization,
) -> Result<LaunchPlan> {
    let kind = classify_prepared_install(prepared)?;
    let availability = inspect_launch_availability(prepared, catalog_database)?;
    let emulator = availability.emulator.ok_or_else(|| {
        anyhow!(
            "No compatible emulator is installed. Install {} and refresh detection.",
            availability.requirement
        )
    })?;
    if emulator.id != expected_emulator_id {
        bail!(
            "the detected prepared-install emulator changed from {expected_emulator_id} to {}; refresh detection before launching",
            emulator.id
        );
    }
    build_plan_for_choice(prepared, kind, emulator, customization)
}

pub fn inspect_rom_launch_availability(
    platform: &str,
    rom_path: &Path,
    catalog_database: &Path,
    preference: Option<&crate::settings::EmulatorPreference>,
) -> Result<RomLaunchAvailability> {
    let host = HostPlatform::current()?;
    let definitions = load_platform_emulator_definitions(catalog_database, host, platform)?;
    let flatpak_apps = installed_flatpak_apps(host);
    let path_entries = executable_search_directories();
    let managed_executables = managed_emulator_executables(host, &path_entries);
    let mut options = Vec::new();

    for definition in &definitions {
        if let Some(choice) = discover_definition(
            &definition.emulator,
            host,
            &path_entries,
            &flatpak_apps,
            &managed_executables,
        ) && standalone_rom_profile_supported(&choice, platform, rom_path)
        {
            options.push(RomEmulatorOption {
                emulator_id: choice.id,
                emulator_name: choice.name,
                runtime_kind: EmulatorRuntimeKind::Standalone,
                core_name: String::new(),
                executable: choice.executable,
                core_path: None,
                recommended: definition.recommended,
            });
        }
        for core in &definition.cores {
            if is_arcade_family_platform(platform) && !is_arcade_archive(rom_path) {
                continue;
            }
            if let Some((executable, core_path)) =
                discover_retroarch_core(core, host, &path_entries, &flatpak_apps)
            {
                options.push(RomEmulatorOption {
                    emulator_id: definition.emulator.id.clone(),
                    emulator_name: definition.emulator.name.clone(),
                    runtime_kind: EmulatorRuntimeKind::RetroArch,
                    core_name: core.clone(),
                    executable,
                    core_path: Some(core_path),
                    recommended: definition.recommended,
                });
            }
        }
    }

    options.sort_by(|left, right| {
        right
            .recommended
            .cmp(&left.recommended)
            .then_with(|| {
                left.runtime_kind
                    .sort_key()
                    .cmp(&right.runtime_kind.sort_key())
            })
            .then_with(|| {
                left.emulator_name
                    .to_ascii_lowercase()
                    .cmp(&right.emulator_name.to_ascii_lowercase())
            })
            .then_with(|| left.core_name.cmp(&right.core_name))
    });
    options.dedup_by(|left, right| {
        left.emulator_id == right.emulator_id
            && left.runtime_kind == right.runtime_kind
            && left.core_name == right.core_name
            && left.executable == right.executable
    });

    let selected_index = preference
        .and_then(|preference| {
            options
                .iter()
                .position(|option| option.matches_preference(preference))
        })
        .or((!options.is_empty()).then_some(0));
    let preference_scope = preference
        .filter(|preference| {
            options
                .iter()
                .any(|option| option.matches_preference(preference))
        })
        .map(|preference| preference.scope.clone())
        .unwrap_or_default();
    let requirement = definitions
        .iter()
        .map(|definition| definition.emulator.name.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(6)
        .collect::<Vec<_>>()
        .join(", ");
    let detail = match selected_index.and_then(|index| options.get(index)) {
        Some(option) if preference_scope.is_empty() => format!(
            "Detected {} compatible emulator option{}; {} is selected automatically.",
            options.len(),
            if options.len() == 1 { "" } else { "s" },
            option.label()
        ),
        Some(option) => format!("Using the {preference_scope} default: {}.", option.label()),
        None if definitions.is_empty() => {
            format!("No emulator catalog entries match {platform} on this host.")
        }
        None => format!(
            "No compatible emulator is installed for {platform}. Install one of: {requirement}."
        ),
    };

    Ok(RomLaunchAvailability {
        options,
        selected_index,
        preference_scope,
        requirement,
        detail,
    })
}

#[cfg(test)]
pub fn build_rom_launch_plan(
    rom_path: &Path,
    platform: &str,
    option: &RomEmulatorOption,
) -> Result<LaunchPlan> {
    build_rom_launch_plan_with_customization(
        rom_path,
        platform,
        option,
        &crate::settings::ResolvedLaunchCustomization::default(),
    )
}

#[cfg(test)]
pub fn build_rom_launch_plan_with_customization(
    rom_path: &Path,
    platform: &str,
    option: &RomEmulatorOption,
    customization: &crate::settings::ResolvedLaunchCustomization,
) -> Result<LaunchPlan> {
    build_rom_launch_plan_with_customization_and_cancellation(
        rom_path,
        platform,
        option,
        customization,
        &Arc::new(AtomicBool::new(false)),
    )
}

pub fn build_rom_launch_plan_with_customization_and_cancellation(
    rom_path: &Path,
    platform: &str,
    option: &RomEmulatorOption,
    customization: &crate::settings::ResolvedLaunchCustomization,
    cancelled: &Arc<AtomicBool>,
) -> Result<LaunchPlan> {
    if !rom_path.is_file() {
        bail!("local game file is missing: {}", rom_path.display());
    }
    let prepared = crate::rom_launch_preparation::prepare_for_launch(
        rom_path,
        is_arcade_family_platform(platform),
        cancelled,
    )?;
    let outcome = build_prepared_rom_launch_plan(
        &prepared.path,
        platform,
        option,
        customization,
        &prepared.access_roots,
    );
    match outcome {
        Ok(mut plan) => {
            plan.cleanup_paths.extend(prepared.cleanup_paths);
            Ok(plan)
        }
        Err(error) => {
            cleanup_after_launch(&prepared.cleanup_paths);
            Err(error)
        }
    }
}

fn build_prepared_rom_launch_plan(
    rom_path: &Path,
    platform: &str,
    option: &RomEmulatorOption,
    customization: &crate::settings::ResolvedLaunchCustomization,
    access_roots: &[PathBuf],
) -> Result<LaunchPlan> {
    let mut current_directory = rom_path
        .parent()
        .map(Path::to_path_buf)
        .context("local game file has no containing directory")?;
    let mut template_values = BTreeMap::from([(
        "file".to_owned(),
        LaunchTemplateValue::Path(rom_path.to_path_buf()),
    )]);

    let (mut arguments, extra_insert_index) = match option.runtime_kind {
        EmulatorRuntimeKind::Standalone => {
            if option.emulator_name.eq_ignore_ascii_case("MAME")
                && is_arcade_family_platform(platform)
            {
                let arguments = mame_arcade_launch_arguments(rom_path, &option.executable)?;
                template_values.insert(
                    "mame_rompath".to_owned(),
                    LaunchTemplateValue::Literal(arguments[1].clone()),
                );
                template_values.insert(
                    "mame_romset".to_owned(),
                    LaunchTemplateValue::Literal(arguments[2].clone()),
                );
                (arguments, 0)
            } else if option.emulator_name.eq_ignore_ascii_case("Hypseus Singe")
                && is_arcade_family_platform(platform)
            {
                let context = hypseus_launch_context(&option.executable, rom_path)?;
                let arguments = context.arguments(&option.executable);
                template_values.insert(
                    "hypseus_game".to_owned(),
                    LaunchTemplateValue::Literal(context.game_name.clone()),
                );
                template_values.insert(
                    "hypseus_framefile".to_owned(),
                    LaunchTemplateValue::Path(context.framefile.clone()),
                );
                template_values.insert(
                    "hypseus_support_root".to_owned(),
                    LaunchTemplateValue::Literal(hypseus_directory_argument(
                        &context.support_root,
                        &option.executable,
                    )),
                );
                template_values.insert(
                    "hypseus_romdir".to_owned(),
                    LaunchTemplateValue::Path(context.rom_directory.clone()),
                );
                current_directory = context.support_root;
                (arguments, 3)
            } else if is_generic_arcade_archive_emulator(&option.emulator_name)
                && is_arcade_family_platform(platform)
                && is_arcade_archive(rom_path)
            {
                (
                    vec![path_argument_for_executable(rom_path, &option.executable)],
                    0,
                )
            } else if is_arcade_family_platform(platform) {
                bail!(
                    "{} does not yet have a safe {} standalone machine profile",
                    platform,
                    option.emulator_name
                );
            } else if option.emulator_name.eq_ignore_ascii_case("Altirra") {
                let media_switch = altirra_media_switch(rom_path);
                template_values.insert(
                    "altirra_media_switch".to_owned(),
                    LaunchTemplateValue::Literal(OsString::from(media_switch)),
                );
                (
                    vec![
                        OsString::from(media_switch),
                        path_argument_for_executable(rom_path, &option.executable),
                    ],
                    0,
                )
            } else {
                (
                    vec![path_argument_for_executable(rom_path, &option.executable)],
                    0,
                )
            }
        }
        EmulatorRuntimeKind::RetroArch => {
            let core_path = option
                .core_path
                .as_deref()
                .context("the selected RetroArch option has no exact core path")?;
            template_values.insert(
                "core".to_owned(),
                LaunchTemplateValue::Path(core_path.to_path_buf()),
            );
            (
                vec![
                    OsString::from("--verbose"),
                    OsString::from("-L"),
                    path_argument_for_executable(core_path, &option.executable),
                    path_argument_for_executable(rom_path, &option.executable),
                ],
                3,
            )
        }
    };

    if customization.command_template.trim().is_empty() {
        let extra_arguments = parse_portable_arguments(&customization.extra_arguments)?
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        arguments.splice(extra_insert_index..extra_insert_index, extra_arguments);
    } else {
        arguments = compile_launch_template(
            customization.command_template.trim(),
            &template_values,
            &option.executable,
        )?;
    }
    let (program, mut prefix_arguments) =
        command_prefix_with_access_roots(&option.executable, &current_directory, access_roots)?;
    prefix_arguments.extend(arguments);

    Ok(LaunchPlan {
        emulator_name: option.label(),
        program,
        arguments: prefix_arguments,
        current_directory,
        environment: launch_environment(&option.executable),
        cleanup_paths: Vec::new(),
    })
}

pub fn spawn_launch_plan(plan: &LaunchPlan) -> Result<Child> {
    let mut command = host_command(&plan.program);
    command
        .args(&plan.arguments)
        .current_dir(&plan.current_directory)
        .envs(plan.environment.iter().cloned())
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
        if path.is_dir() && crate::rom_launch_preparation::cleanup_owned_path(path) {
            let _ = fs::remove_dir_all(path);
        } else {
            let _ = fs::remove_file(path);
        }
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
             WHERE emulator_id=?1 AND host_system_slug=?2 ORDER BY manager, package_id",
        )?;
        let rows = packages.query_map([definition.id.as_str(), host.catalog_slug()], |row| {
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

fn load_platform_emulator_definitions(
    database: &Path,
    host: HostPlatform,
    platform: &str,
) -> Result<Vec<PlatformEmulatorDefinition>> {
    let platform_key = emulator_catalog_platform_key(platform);
    if platform_key.is_empty() {
        bail!("a platform is required for emulator discovery");
    }
    let connection = crate::catalog::open_read_only(database, "Lunchbox emulator catalog")?;
    let mut statement = connection.prepare(
        "SELECT e.id, e.name, ep.core_name, ep.recommended
         FROM emulator_platforms ep
         JOIN emulators e ON e.id=ep.emulator_id
         JOIN platforms p ON p.id=ep.platform_id
         JOIN emulator_host_systems h ON h.emulator_id=e.id
         WHERE h.host_system_slug=?1
           AND (
               p.normalized_name=?2 OR EXISTS (
                   SELECT 1 FROM platform_aliases a
                   WHERE a.platform_id=p.id AND a.normalized_alias=?2
               )
           )
         ORDER BY ep.recommended DESC, e.name COLLATE NOCASE, ep.core_name",
    )?;
    let rows = statement.query_map(
        rusqlite::params![host.catalog_slug(), platform_key],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
            ))
        },
    )?;
    let mut definitions = BTreeMap::<String, PlatformEmulatorDefinition>::new();
    for row in rows {
        let (id, name, core_names, recommended) = row?;
        let definition =
            definitions
                .entry(id.clone())
                .or_insert_with(|| PlatformEmulatorDefinition {
                    emulator: EmulatorDefinition {
                        id,
                        name,
                        packages: BTreeMap::new(),
                    },
                    cores: Vec::new(),
                    recommended: false,
                });
        definition.recommended |= recommended;
        definition.cores.extend(
            core_names
                .split(';')
                .map(str::trim)
                .filter(|core| !core.is_empty())
                .map(ToOwned::to_owned),
        );
    }

    for definition in definitions.values_mut() {
        definition.cores.sort();
        definition.cores.dedup();
        load_emulator_packages(&connection, host, &mut definition.emulator)?;
    }
    Ok(definitions.into_values().collect())
}

fn load_emulator_packages(
    connection: &rusqlite::Connection,
    host: HostPlatform,
    definition: &mut EmulatorDefinition,
) -> Result<()> {
    let mut packages = connection.prepare(
        "SELECT manager, package_id FROM emulator_packages
         WHERE emulator_id=?1 AND host_system_slug=?2 ORDER BY manager, package_id",
    )?;
    let rows = packages.query_map([definition.id.as_str(), host.catalog_slug()], |row| {
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
    Ok(())
}

fn discover_definition(
    definition: &EmulatorDefinition,
    host: HostPlatform,
    path_entries: &[PathBuf],
    flatpak_apps: &BTreeSet<String>,
    managed_executables: &BTreeMap<String, EmulatorExecutable>,
) -> Option<EmulatorChoice> {
    let mut native_directories = platform_install_directories(definition, host);
    native_directories.extend_from_slice(path_entries);
    let executable = executable_names(&definition.name, host)
        .into_iter()
        .find_map(|name| find_executable_in_paths(&name, &native_directories))
        .map(EmulatorExecutable::Native)
        .or_else(|| managed_executables.get(&definition.id).cloned())
        .or_else(|| discover_macos_application(definition, host))
        .or_else(|| {
            let flatpak_command = find_executable_in_paths("flatpak", path_entries)?;
            definition
                .packages
                .get("flatpak")?
                .iter()
                .filter(|app_id| {
                    definition.name.eq_ignore_ascii_case("RetroArch")
                        || !app_id.eq_ignore_ascii_case("org.libretro.RetroArch")
                })
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

fn discover_retroarch_core(
    core_name: &str,
    host: HostPlatform,
    path_entries: &[PathBuf],
    flatpak_apps: &BTreeSet<String>,
) -> Option<(EmulatorExecutable, PathBuf)> {
    let native = ["retroarch", "RetroArch"]
        .into_iter()
        .find_map(|name| find_executable_in_paths(name, path_entries))
        .or_else(|| {
            discover_macos_application(
                &EmulatorDefinition {
                    id: "retroarch".to_owned(),
                    name: "RetroArch".to_owned(),
                    packages: BTreeMap::new(),
                },
                host,
            )
            .and_then(|executable| match executable {
                EmulatorExecutable::Native(path) => Some(path),
                EmulatorExecutable::Flatpak { .. } | EmulatorExecutable::Wine { .. } => None,
            })
        });
    if let Some(native) = native {
        let mut core_directories = native_retroarch_core_directories(host);
        if let Some(parent) = native.parent() {
            core_directories.insert(0, parent.join("cores"));
        }
        if let Some(core_path) = find_retroarch_core(core_name, host, &core_directories) {
            return Some((EmulatorExecutable::Native(native), core_path));
        }
    }

    if host == HostPlatform::Linux
        && flatpak_apps.contains("org.libretro.RetroArch")
        && let Some(command) = find_executable_in_paths("flatpak", path_entries)
        && let Some(core_path) =
            find_retroarch_core(core_name, host, &flatpak_retroarch_core_directories())
    {
        return Some((
            EmulatorExecutable::Flatpak {
                command,
                app_id: "org.libretro.RetroArch".to_owned(),
            },
            core_path,
        ));
    }
    None
}

fn find_retroarch_core(
    core_name: &str,
    host: HostPlatform,
    directories: &[PathBuf],
) -> Option<PathBuf> {
    let suffix = match host {
        HostPlatform::Linux => "so",
        HostPlatform::Windows => "dll",
        HostPlatform::MacOs => "dylib",
    };
    let filename = format!("{core_name}_libretro.{suffix}");
    directories
        .iter()
        .map(|directory| directory.join(&filename))
        .find(|path| path.is_file())
}

fn native_retroarch_core_directories(host: HostPlatform) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    let base_dirs = directories::BaseDirs::new();
    match host {
        HostPlatform::Linux => {
            if let Some(base_dirs) = &base_dirs {
                directories.push(base_dirs.home_dir().join(".config/retroarch/cores"));
            }
            directories.extend([
                PathBuf::from("/run/current-system/sw/lib/libretro"),
                PathBuf::from("/usr/lib/libretro"),
                PathBuf::from("/usr/lib64/libretro"),
            ]);
        }
        HostPlatform::Windows => {
            if let Some(local) = env::var_os("LOCALAPPDATA") {
                directories.push(PathBuf::from(local).join("RetroArch/cores"));
            }
            if let Some(program_files) = env::var_os("ProgramFiles") {
                directories.push(PathBuf::from(program_files).join("RetroArch/cores"));
            }
        }
        HostPlatform::MacOs => {
            directories.push(PathBuf::from(
                "/Applications/RetroArch.app/Contents/Resources/cores",
            ));
            if let Some(base_dirs) = &base_dirs {
                directories.push(
                    base_dirs
                        .home_dir()
                        .join("Library/Application Support/RetroArch/cores"),
                );
            }
        }
    }
    directories
}

fn flatpak_retroarch_core_directories() -> Vec<PathBuf> {
    directories::BaseDirs::new()
        .map(|base_dirs| {
            vec![
                base_dirs
                    .home_dir()
                    .join(".var/app/org.libretro.RetroArch/config/retroarch/cores"),
            ]
        })
        .unwrap_or_default()
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

fn managed_emulator_executables(
    host: HostPlatform,
    path_entries: &[PathBuf],
) -> BTreeMap<String, EmulatorExecutable> {
    let Some(project) = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox") else {
        return BTreeMap::new();
    };
    let Ok(store) = crate::settings::SettingsStore::open_default() else {
        return BTreeMap::new();
    };
    let Ok(receipts) = store.managed_emulator_installs() else {
        return BTreeMap::new();
    };
    let wine = (host == HostPlatform::Linux)
        .then(|| {
            ["wine64", "wine"]
                .into_iter()
                .find_map(|name| find_executable_in_paths(name, path_entries))
        })
        .flatten();
    let mut executables = BTreeMap::new();
    for receipt in receipts {
        if receipt.host_system_slug != host.catalog_slug()
            || !matches!(receipt.manager.as_str(), "appimage" | "github" | "direct")
            || receipt.emulator_id.is_empty()
            || !receipt
                .emulator_id
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            continue;
        }
        let path = PathBuf::from(&receipt.install_path);
        let owned_root = project
            .data_local_dir()
            .join("programs")
            .join(&receipt.manager)
            .join(&receipt.emulator_id);
        if !path.is_file() || !path.starts_with(&owned_root) {
            continue;
        }
        let executable = if receipt.manager == "direct" {
            let Some(command) = wine.clone() else {
                continue;
            };
            EmulatorExecutable::Wine {
                command,
                executable: path,
                prefix: project
                    .data_local_dir()
                    .join("wine-prefixes")
                    .join(&receipt.emulator_id),
            }
        } else if executable_file(&path) {
            EmulatorExecutable::Native(path)
        } else {
            continue;
        };
        executables.insert(receipt.emulator_id, executable);
    }
    executables
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
        "atari++" => &["ataripp"],
        "desmume" => &["desmume", "DeSmuME", "desmume-gtk"],
        "dolphin" => &["dolphin-emu", "dolphin-emu-qt", "dolphin"],
        "dosbox staging" => &["dosbox", "dosbox-staging"],
        "dosbox-x" => &["dosbox-x", "DOSBox-X"],
        "duckstation" => &["duckstation-qt", "duckstation-nogui", "duckstation"],
        "fs-uae" => &["fs-uae", "fs-uae-launcher"],
        "hypseus singe" => &["hypseus", "singe"],
        "mesen" => &["Mesen", "mesen", "mesen-x"],
        "ppsspp" => &["PPSSPP", "PPSSPPQt", "ppsspp"],
        "scummvm" => &["scummvm", "ScummVM"],
        "vice" => &["x64sc", "x64", "x128", "xplus4", "vice"],
        "vice (xpet)" => &["xpet"],
        "vice (xvic)" => &["xvic"],
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
    let flatpak = find_executable_in_paths("flatpak", &paths)
        .or_else(|| is_flatpak().then(|| PathBuf::from("flatpak")));
    let Some(flatpak) = flatpak else {
        return BTreeSet::new();
    };
    host_command(flatpak)
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
    customization: &crate::settings::ResolvedLaunchCustomization,
) -> Result<LaunchPlan> {
    let mut cleanup_paths = Vec::new();
    let mut template_values = BTreeMap::new();
    let (mut arguments, extra_insert_index) = match kind {
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
            template_values.insert("config".to_owned(), LaunchTemplateValue::Path(config_path));
            if let Some(options) = shared_options_path {
                arguments.push(OsString::from("-conf"));
                arguments.push(path_argument(&options, &emulator));
                template_values.insert(
                    "shared_config".to_owned(),
                    LaunchTemplateValue::Path(options),
                );
            }
            if is_dosbox_x(&emulator) {
                // DOSBox-X otherwise prompts for a working directory on first
                // run; the plan already starts it inside the prepared install.
                arguments.push(OsString::from("-set"));
                arguments.push(OsString::from("dosbox working directory option=noprompt"));
            }
            let extra_insert_index = arguments.len();
            (arguments, extra_insert_index)
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
            template_values.insert(
                "config".to_owned(),
                LaunchTemplateValue::Path(config_path.clone()),
            );
            template_values.insert(
                "game_path".to_owned(),
                LaunchTemplateValue::Path(game_path.clone()),
            );
            template_values.insert(
                "game_id".to_owned(),
                LaunchTemplateValue::Literal(OsString::from(&plan.game_id)),
            );
            let mut arguments = vec![
                OsString::from("--config"),
                path_argument(&config_path, &emulator),
            ];
            arguments.extend(plan.extra_args.into_iter().map(OsString::from));
            let extra_insert_index = arguments.len();
            arguments.push(OsString::from("-p"));
            arguments.push(path_argument(&game_path, &emulator));
            arguments.push(OsString::from(plan.game_id));
            (arguments, extra_insert_index)
        }
        PreparedLaunchKind::Win9xDosboxX {
            config_path,
            shared_options_path,
        } => {
            template_values.insert(
                "config".to_owned(),
                LaunchTemplateValue::Path(config_path.clone()),
            );
            template_values.insert(
                "shared_config".to_owned(),
                LaunchTemplateValue::Path(shared_options_path.clone()),
            );
            let mut arguments = vec![
                OsString::from("-conf"),
                path_argument(&config_path, &emulator),
                OsString::from("-conf"),
                path_argument(&shared_options_path, &emulator),
                OsString::from("-nomenu"),
                OsString::from("-noconsole"),
            ];
            if is_dosbox_x(&emulator) {
                arguments.push(OsString::from("-set"));
                arguments.push(OsString::from("dosbox working directory option=noprompt"));
            }
            let extra_insert_index = arguments.len();
            (arguments, extra_insert_index)
        }
        PreparedLaunchKind::EightySixBox(plan) => {
            let vm_root = prepare_86box_vm(prepared, &plan)?;
            template_values.insert(
                "vm_root".to_owned(),
                LaunchTemplateValue::Path(vm_root.clone()),
            );
            (
                vec![OsString::from("-P"), path_argument(&vm_root, &emulator)],
                2,
            )
        }
        PreparedLaunchKind::PcBox(plan) => {
            let native_pcbox = emulator.name.eq_ignore_ascii_case("PCBox");
            let (vm_root, config_path) = prepare_pcbox_vm(prepared, &plan, native_pcbox)?;
            template_values.insert(
                "vm_root".to_owned(),
                LaunchTemplateValue::Path(vm_root.clone()),
            );
            template_values.insert(
                "config".to_owned(),
                LaunchTemplateValue::Path(config_path.clone()),
            );
            if native_pcbox {
                if matches!(emulator.executable, EmulatorExecutable::Flatpak { .. }) {
                    bail!("PCBox prepared installs currently require a native PCBox executable")
                }
                (vec![OsString::from("-c"), config_path.into_os_string()], 2)
            } else {
                (
                    vec![OsString::from("-P"), path_argument(&vm_root, &emulator)],
                    2,
                )
            }
        }
    };

    let customized_arguments = if customization.command_template.trim().is_empty() {
        let extra_arguments = parse_portable_arguments(&customization.extra_arguments)?
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        arguments.splice(extra_insert_index..extra_insert_index, extra_arguments);
        Ok(arguments)
    } else {
        compile_launch_template(
            customization.command_template.trim(),
            &template_values,
            &emulator.executable,
        )
    };
    let arguments = match customized_arguments {
        Ok(arguments) => arguments,
        Err(error) => {
            cleanup_after_launch(&cleanup_paths);
            return Err(error);
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
        environment: launch_environment(&emulator.executable),
        cleanup_paths,
    })
}

fn command_prefix(
    executable: &EmulatorExecutable,
    install_root: &Path,
) -> Result<(PathBuf, Vec<OsString>)> {
    command_prefix_with_access_roots(executable, install_root, &[])
}

fn is_dosbox_x(emulator: &EmulatorChoice) -> bool {
    emulator.name.eq_ignore_ascii_case("DOSBox-X")
}

fn command_prefix_with_access_roots(
    executable: &EmulatorExecutable,
    install_root: &Path,
    access_roots: &[PathBuf],
) -> Result<(PathBuf, Vec<OsString>)> {
    match executable {
        EmulatorExecutable::Native(path) => Ok((path.clone(), Vec::new())),
        EmulatorExecutable::Flatpak { command, app_id } => {
            let mut mounts = BTreeSet::new();
            mounts.insert(map_path_for_flatpak(&flatpak_mount_point(install_root)?));
            for access_root in access_roots {
                mounts.insert(map_path_for_flatpak(&flatpak_mount_point(access_root)?));
            }
            let mut arguments = vec![OsString::from("run")];
            arguments.extend(
                mounts
                    .into_iter()
                    .map(|mount| OsString::from(format!("--filesystem={}", mount.display()))),
            );
            arguments.push(OsString::from(app_id));
            Ok((command.clone(), arguments))
        }
        EmulatorExecutable::Wine {
            command,
            executable,
            ..
        } => Ok((command.clone(), vec![executable.as_os_str().to_owned()])),
    }
}

fn launch_environment(executable: &EmulatorExecutable) -> Vec<(OsString, OsString)> {
    match executable {
        EmulatorExecutable::Wine { prefix, .. } => {
            vec![(OsString::from("WINEPREFIX"), prefix.as_os_str().to_owned())]
        }
        EmulatorExecutable::Native(_) | EmulatorExecutable::Flatpak { .. } => Vec::new(),
    }
}

fn path_argument(path: &Path, emulator: &EmulatorChoice) -> OsString {
    path_argument_for_executable(path, &emulator.executable)
}

fn path_argument_for_executable(path: &Path, executable: &EmulatorExecutable) -> OsString {
    match executable {
        EmulatorExecutable::Flatpak { .. } => map_path_for_flatpak(path).into_os_string(),
        EmulatorExecutable::Native(_) => path.as_os_str().to_owned(),
        EmulatorExecutable::Wine { .. } => map_path_for_wine(path),
    }
}

fn map_path_for_wine(path: &Path) -> OsString {
    if !path.is_absolute() {
        return path.as_os_str().to_owned();
    }
    let mut windows = String::from("Z:");
    windows.push_str(&path.to_string_lossy().replace('/', "\\"));
    OsString::from(windows)
}

fn emulator_catalog_platform_key(platform: &str) -> String {
    let key = crate::catalog::normalize_platform_key(platform);
    if matches!(key.as_str(), "arcade-pinball" | "arcade-laserdisc") {
        "arcade".to_owned()
    } else {
        key
    }
}

fn is_arcade_family_platform(platform: &str) -> bool {
    matches!(
        crate::catalog::normalize_platform_key(platform).as_str(),
        "arcade" | "arcade-pinball" | "arcade-laserdisc"
    )
}

fn is_arcade_archive(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("zip") || extension.eq_ignore_ascii_case("7z")
        })
}

fn standalone_rom_profile_supported(
    emulator: &EmulatorChoice,
    platform: &str,
    rom_path: &Path,
) -> bool {
    if !is_arcade_family_platform(platform) {
        return true;
    }
    if emulator.name.eq_ignore_ascii_case("MAME") {
        return is_arcade_archive(rom_path);
    }
    if emulator.name.eq_ignore_ascii_case("Hypseus Singe") {
        return hypseus_launch_context(&emulator.executable, rom_path).is_ok();
    }
    is_generic_arcade_archive_emulator(&emulator.name) && is_arcade_archive(rom_path)
}

fn is_generic_arcade_archive_emulator(name: &str) -> bool {
    ["FinalBurn Neo", "Flycast", "Supermodel"]
        .into_iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn altirra_media_switch(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("atr" | "atx" | "xfd" | "dcm" | "pro" | "atz") => "/disk",
        Some("cas" | "wav") => "/tape",
        Some("car" | "rom" | "bin" | "a52") => "/cart",
        _ => "/run",
    }
}

fn mame_arcade_launch_arguments(
    rom_path: &Path,
    executable: &EmulatorExecutable,
) -> Result<Vec<OsString>> {
    if !is_arcade_archive(rom_path) {
        bail!(
            "MAME arcade launch requires a ZIP or 7z ROM-set archive, not {}",
            rom_path.display()
        );
    }
    let romset = rom_path
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .context("MAME ROM-set archive has no set name")?;
    let rom_parent = rom_path
        .parent()
        .context("MAME ROM-set archive has no containing directory")?;
    let mut rompath = path_argument_for_executable(rom_parent, executable);
    if let Some(runtime) = mame_runtime_roms_directory(executable) {
        let runtime = path_argument_for_executable(&runtime, executable);
        if runtime != rompath {
            rompath.push(";");
            rompath.push(runtime);
        }
    }
    Ok(vec![OsString::from("-rompath"), rompath, romset.to_owned()])
}

fn mame_runtime_roms_directory(executable: &EmulatorExecutable) -> Option<PathBuf> {
    let base_dirs = directories::BaseDirs::new()?;
    match executable {
        EmulatorExecutable::Flatpak { app_id, .. }
            if app_id.eq_ignore_ascii_case("org.mamedev.MAME") =>
        {
            Some(
                base_dirs
                    .home_dir()
                    .join(".var/app/org.mamedev.MAME/data/mame/roms"),
            )
        }
        EmulatorExecutable::Flatpak { .. } => None,
        EmulatorExecutable::Native(_) => {
            if cfg!(target_os = "windows") {
                Some(base_dirs.data_local_dir().join("mame/roms"))
            } else {
                Some(base_dirs.home_dir().join(".mame/roms"))
            }
        }
        EmulatorExecutable::Wine { .. } => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HypseusLaunchContext {
    game_name: OsString,
    framefile: PathBuf,
    support_root: PathBuf,
    rom_directory: PathBuf,
}

impl HypseusLaunchContext {
    fn arguments(&self, executable: &EmulatorExecutable) -> Vec<OsString> {
        let support_root = hypseus_directory_argument(&self.support_root, executable);
        vec![
            self.game_name.clone(),
            OsString::from("vldp"),
            OsString::from("-fullscreen"),
            OsString::from("-framefile"),
            path_argument_for_executable(&self.framefile, executable),
            OsString::from("-homedir"),
            support_root.clone(),
            OsString::from("-datadir"),
            support_root,
            OsString::from("-romdir"),
            path_argument_for_executable(&self.rom_directory, executable),
        ]
    }
}

fn hypseus_launch_context(
    executable: &EmulatorExecutable,
    framefile: &Path,
) -> Result<HypseusLaunchContext> {
    if !framefile.is_file()
        || !framefile
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
    {
        bail!(
            "Hypseus Singe requires a present laserdisc framefile, not {}",
            framefile.display()
        );
    }
    let game_name = framefile
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .context("Hypseus framefile has no game name")?
        .to_owned();
    let bundle_root = framefile
        .ancestors()
        .skip(1)
        .find(|ancestor| {
            ancestor
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| {
                    name.eq_ignore_ascii_case("vldp") || name.eq_ignore_ascii_case("singe")
                })
        })
        .and_then(Path::parent)
        .context("Hypseus framefile is not inside a vldp or singe bundle")?
        .to_path_buf();
    let rom_directory = bundle_root.join("roms");
    if !rom_directory.is_dir() {
        bail!(
            "Hypseus laserdisc bundle is missing ROM directory {}",
            rom_directory.display()
        );
    }
    let support_root = resolve_hypseus_support_root(executable)?;
    Ok(HypseusLaunchContext {
        game_name,
        framefile: framefile.to_path_buf(),
        support_root,
        rom_directory,
    })
}

fn resolve_hypseus_support_root(executable: &EmulatorExecutable) -> Result<PathBuf> {
    let EmulatorExecutable::Native(executable) = executable else {
        bail!("Hypseus Flatpak support assets are not yet represented by the emulator catalog");
    };
    let executable_parent = executable
        .parent()
        .context("Hypseus executable has no containing directory")?;
    let mut candidates = vec![executable_parent.to_path_buf()];
    if cfg!(target_os = "macos")
        && let Some(contents) = executable_parent.parent()
    {
        candidates.push(contents.join("Resources"));
    }
    if let Some(prefix) = executable_parent.parent() {
        candidates.push(prefix.join("share/hypseus-singe"));
        candidates.push(prefix.join("share/hypseus"));
    }
    candidates.extend([
        PathBuf::from("/usr/local/share/hypseus-singe"),
        PathBuf::from("/usr/share/hypseus-singe"),
    ]);
    candidates.sort();
    candidates.dedup();
    candidates
        .into_iter()
        .find(|candidate| hypseus_support_tree_complete(candidate))
        .with_context(|| {
            format!(
                "Hypseus support assets were not found beside {} or in a platform share directory",
                executable.display()
            )
        })
}

fn hypseus_support_tree_complete(directory: &Path) -> bool {
    directory.join("pics/overlayleds2.bmp").is_file()
        && directory.join("fonts/default.ttf").is_file()
        && directory.join("hypinput.ini").is_file()
}

fn hypseus_directory_argument(directory: &Path, executable: &EmulatorExecutable) -> OsString {
    let mut argument = path_argument_for_executable(directory, executable);
    if !directory.as_os_str().is_empty() {
        argument.push(std::path::MAIN_SEPARATOR_STR);
    }
    argument
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
    use std::fs::File;
    use std::io::Write;
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
    fn portable_launch_arguments_preserve_windows_paths_without_a_shell() {
        assert_eq!(
            parse_portable_arguments(
                r#"--fullscreen "two words" C:\Games\ROMs\game.zip 'literal value' "" "\\server\ROM Share\game.zip" \\server\roms\game.zip"#
            )
            .unwrap(),
            [
                "--fullscreen",
                "two words",
                r"C:\Games\ROMs\game.zip",
                "literal value",
                "",
                r"\\server\ROM Share\game.zip",
                r"\\server\roms\game.zip",
            ]
        );
        assert!(parse_portable_arguments("'unterminated").is_err());
        assert!(validate_launch_template("%{unknown} %f").is_err());
        validate_launch_template("--file=%f %% %{core}").unwrap();
    }

    #[test]
    fn launch_preview_uses_runtime_argv_semantics_without_shell_rendering() {
        let available = vec!["core".to_owned(), "file".to_owned()];
        let built_in = preview_launch_command(
            "RetroArch",
            "--verbose -L %{core} %f",
            r#"--label "Living Room""#,
            "",
            &available,
            3,
        )
        .unwrap();
        assert_eq!(built_in.runtime, "RetroArch");
        assert_eq!(
            built_in.arguments,
            [
                "--verbose",
                "-L",
                "<retroarch-core>",
                "--label",
                "Living Room",
                "<selected-file>",
            ]
        );
        assert!(!built_in.uses_custom_template);
        assert!(!built_in.extra_arguments_ignored);

        let custom = preview_launch_command(
            "RetroArch",
            "--verbose -L %{core} %f",
            "--ignored",
            r#"--file %f "$(never-executed)""#,
            &available,
            3,
        )
        .unwrap();
        assert_eq!(
            custom.arguments,
            ["--file", "<selected-file>", "$(never-executed)"]
        );
        assert!(custom.uses_custom_template);
        assert!(custom.extra_arguments_ignored);
        assert!(custom.summary().contains("ignored"));
    }

    #[test]
    fn launch_preview_rejects_invalid_or_contextually_unavailable_drafts() {
        let available = vec!["file".to_owned()];
        assert!(
            preview_launch_command("Emulator", "%f", "'unterminated", "", &available, 0).is_err()
        );
        let error =
            preview_launch_command("Emulator", "%f", "", "%{core} %f", &available, 0).unwrap_err();
        assert!(error.to_string().contains("unavailable"));
    }

    #[test]
    fn launch_template_replaces_argv_and_extra_arguments_augment_defaults() {
        let temp = TempDir::new().unwrap();
        let rom = temp.path().join("Game with spaces.rom");
        fs::write(&rom, b"rom").unwrap();
        let option = RomEmulatorOption {
            emulator_id: "native-id".to_owned(),
            emulator_name: "Native Emulator".to_owned(),
            runtime_kind: EmulatorRuntimeKind::Standalone,
            core_name: String::new(),
            executable: EmulatorExecutable::Native(PathBuf::from("/bin/emulator")),
            core_path: None,
            recommended: true,
        };

        let augmented = build_rom_launch_plan_with_customization(
            &rom,
            "Example",
            &option,
            &crate::settings::ResolvedLaunchCustomization {
                extra_arguments: "--fullscreen --label 'Living Room'".to_owned(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            augmented.arguments,
            [
                OsString::from("--fullscreen"),
                OsString::from("--label"),
                OsString::from("Living Room"),
                rom.as_os_str().to_owned(),
            ]
        );

        let replaced = build_rom_launch_plan_with_customization(
            &rom,
            "Example",
            &option,
            &crate::settings::ResolvedLaunchCustomization {
                extra_arguments: "--ignored".to_owned(),
                command_template: "--file %f '$(never-executed)'".to_owned(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            replaced.arguments,
            [
                OsString::from("--file"),
                rom.as_os_str().to_owned(),
                OsString::from("$(never-executed)"),
            ]
        );
    }

    #[test]
    fn prepared_launch_profiles_preserve_exact_default_and_replacement_semantics() {
        let temp = TempDir::new().unwrap();
        let prepared = prepared(
            temp.path(),
            ExoCollection::Dos,
            "eXoDOS/!dos/TEST/dosbox_linux.conf",
        );
        let shared = temp.path().join("emulators/dosbox/options_linux.conf");
        fs::create_dir_all(shared.parent().unwrap()).unwrap();
        fs::write(&shared, b"options").unwrap();
        let choice = EmulatorChoice {
            id: "dosbox-x".to_owned(),
            name: "DOSBox-X".to_owned(),
            executable: EmulatorExecutable::Native(PathBuf::from("/bin/dosbox-x")),
        };

        let kind = classify_prepared_install(&prepared).unwrap();
        assert_eq!(
            prepared_launch_template(&kind, &choice.name),
            "-conf %{config} -conf %{shared_config}"
        );
        let augmented = build_plan_for_choice(
            &prepared,
            kind.clone(),
            choice.clone(),
            &crate::settings::ResolvedLaunchCustomization {
                extra_arguments: "--fullscreen --label 'Living Room'".to_owned(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            augmented.arguments,
            [
                OsString::from("-conf"),
                prepared.launch_config_path.as_os_str().to_owned(),
                OsString::from("-conf"),
                shared.as_os_str().to_owned(),
                OsString::from("-set"),
                OsString::from("dosbox working directory option=noprompt"),
                OsString::from("--fullscreen"),
                OsString::from("--label"),
                OsString::from("Living Room"),
            ]
        );

        let replaced = build_plan_for_choice(
            &prepared,
            kind,
            choice,
            &crate::settings::ResolvedLaunchCustomization {
                extra_arguments: "--ignored".to_owned(),
                command_template: "%{config} '--safe=$(never-executed)'".to_owned(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            replaced.arguments,
            [
                prepared.launch_config_path.into_os_string(),
                OsString::from("--safe=$(never-executed)"),
            ]
        );
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
                 CREATE TABLE emulator_packages(
                   emulator_id TEXT, host_system_slug TEXT, manager TEXT, package_id TEXT
                 );
                 INSERT INTO emulators VALUES('dosbox-x-id','DOSBox-X');
                 INSERT INTO emulators VALUES('wrong-host','DOSBox Staging');
                 INSERT INTO emulator_host_systems VALUES('dosbox-x-id','linux');
                 INSERT INTO emulator_host_systems VALUES('wrong-host','windows');
                 INSERT INTO emulator_packages VALUES(
                   'dosbox-x-id','linux','flatpak','com.dosbox_x.DOSBox-X'
                 );",
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
    fn canonical_platform_alias_drives_exact_core_and_recommendation() {
        let temp = TempDir::new().unwrap();
        let database = temp.path().join("catalog.db");
        let connection = Connection::open(&database).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE emulators(id TEXT PRIMARY KEY, name TEXT);
                 CREATE TABLE emulator_host_systems(emulator_id TEXT, host_system_slug TEXT);
                 CREATE TABLE emulator_packages(
                   emulator_id TEXT, host_system_slug TEXT, manager TEXT, package_id TEXT
                 );
                 CREATE TABLE platforms(id TEXT PRIMARY KEY, normalized_name TEXT);
                 CREATE TABLE platform_aliases(platform_id TEXT, normalized_alias TEXT);
                 CREATE TABLE emulator_platforms(
                   emulator_id TEXT, platform_id TEXT, core_name TEXT, recommended INTEGER
                 );
                 INSERT INTO emulators VALUES('mesen-id','Mesen');
                 INSERT INTO emulator_host_systems VALUES('mesen-id','linux');
                 INSERT INTO emulator_packages VALUES(
                   'mesen-id','linux','flatpak','dev.mesen.Mesen'
                 );
                 INSERT INTO platforms VALUES('nes-id','nintendo entertainment system');
                 INSERT INTO platform_aliases VALUES('nes-id','nes');
                 INSERT INTO emulator_platforms VALUES('mesen-id','nes-id','mesen',1);",
            )
            .unwrap();
        drop(connection);

        let definitions =
            load_platform_emulator_definitions(&database, HostPlatform::Linux, "NES").unwrap();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].emulator.id, "mesen-id");
        assert_eq!(definitions[0].cores, ["mesen"]);
        assert!(definitions[0].recommended);
        assert_eq!(
            definitions[0].emulator.packages["flatpak"],
            ["dev.mesen.Mesen"]
        );
        assert_eq!(emulator_catalog_platform_key("Arcade Laserdisc"), "arcade");
        assert_eq!(emulator_catalog_platform_key("Arcade Pinball"), "arcade");
        assert!(is_arcade_family_platform("arcade-laserdisc"));
    }

    #[test]
    fn native_retroarch_plan_passes_exact_core_and_rom_without_a_shell() {
        let temp = TempDir::new().unwrap();
        let rom = temp.path().join("Game (USA).nes");
        let core = temp.path().join("mesen_libretro.so");
        fs::write(&rom, b"rom").unwrap();
        fs::write(&core, b"core").unwrap();
        let option = RomEmulatorOption {
            emulator_id: "mesen-id".to_owned(),
            emulator_name: "Mesen".to_owned(),
            runtime_kind: EmulatorRuntimeKind::RetroArch,
            core_name: "mesen".to_owned(),
            executable: EmulatorExecutable::Native(PathBuf::from("/usr/bin/retroarch")),
            core_path: Some(core.clone()),
            recommended: true,
        };

        let plan = build_rom_launch_plan(&rom, "Nintendo Entertainment System", &option).unwrap();
        assert_eq!(plan.program, Path::new("/usr/bin/retroarch"));
        assert_eq!(
            plan.arguments,
            [
                OsString::from("--verbose"),
                OsString::from("-L"),
                core.into_os_string(),
                rom.into_os_string(),
            ]
        );
        assert!(!plan.arguments.iter().any(|argument| argument == "-c"));
    }

    #[test]
    fn flatpak_retroarch_plan_grants_only_the_rom_directory() {
        let temp = TempDir::new().unwrap();
        let rom = temp.path().join("Game.nes");
        let core = temp.path().join("mesen_libretro.so");
        fs::write(&rom, b"rom").unwrap();
        fs::write(&core, b"core").unwrap();
        let option = RomEmulatorOption {
            emulator_id: "mesen-id".to_owned(),
            emulator_name: "Mesen".to_owned(),
            runtime_kind: EmulatorRuntimeKind::RetroArch,
            core_name: "mesen".to_owned(),
            executable: EmulatorExecutable::Flatpak {
                command: PathBuf::from("/usr/bin/flatpak"),
                app_id: "org.libretro.RetroArch".to_owned(),
            },
            core_path: Some(core.clone()),
            recommended: true,
        };

        let plan = build_rom_launch_plan(&rom, "Nintendo Entertainment System", &option).unwrap();
        assert_eq!(plan.program, Path::new("/usr/bin/flatpak"));
        assert_eq!(plan.arguments[0], "run");
        assert_eq!(
            plan.arguments[1],
            OsString::from(format!("--filesystem={}", temp.path().display()))
        );
        assert_eq!(plan.arguments[2], "org.libretro.RetroArch");
        assert_eq!(plan.arguments[3], "--verbose");
        assert_eq!(plan.arguments[4], "-L");
        assert_eq!(plan.arguments[5], core.as_os_str());
        assert_eq!(plan.arguments[6], rom.as_os_str());
    }

    #[test]
    fn multi_disc_archive_playlist_is_prepared_and_cleaned_after_launch() {
        let temp = TempDir::new().unwrap();
        let archive_path = temp.path().join("Disc 1.zip");
        let file = File::create(&archive_path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                "Game (Disc 1).cue",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive
            .write_all(b"FILE \"Game (Disc 1).bin\" BINARY\n")
            .unwrap();
        archive
            .start_file(
                "Game (Disc 1).bin",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"disc").unwrap();
        archive.finish().unwrap();
        let playlist = temp.path().join("Game.m3u");
        fs::write(&playlist, "Disc 1.zip\n").unwrap();
        let option = RomEmulatorOption {
            emulator_id: "native-id".to_owned(),
            emulator_name: "Native Emulator".to_owned(),
            runtime_kind: EmulatorRuntimeKind::Standalone,
            core_name: String::new(),
            executable: EmulatorExecutable::Native(PathBuf::from("/bin/emulator")),
            core_path: None,
            recommended: true,
        };

        let plan = build_rom_launch_plan(&playlist, "Sony PlayStation", &option).unwrap();
        assert_eq!(plan.cleanup_paths.len(), 1);
        let generated = PathBuf::from(&plan.arguments[0]);
        assert!(generated.is_file());
        assert!(
            fs::read_to_string(&generated)
                .unwrap()
                .contains("Game (Disc 1).cue")
        );
        let session = plan.cleanup_paths[0].clone();
        cleanup_after_launch(&plan.cleanup_paths);
        assert!(!session.exists());
    }

    #[test]
    fn flatpak_playlist_mounts_each_external_disc_directory() {
        let temp = TempDir::new().unwrap();
        let playlist_directory = temp.path().join("playlists");
        let disc_directory = temp.path().join("discs");
        fs::create_dir_all(&playlist_directory).unwrap();
        fs::create_dir_all(&disc_directory).unwrap();
        let disc = disc_directory.join("Game (Disc 1).chd");
        fs::write(&disc, b"disc").unwrap();
        let playlist = playlist_directory.join("Game.m3u");
        fs::write(&playlist, format!("{}\n", disc.display())).unwrap();
        let option = RomEmulatorOption {
            emulator_id: "retroarch-id".to_owned(),
            emulator_name: "Beetle PSX".to_owned(),
            runtime_kind: EmulatorRuntimeKind::RetroArch,
            core_name: "mednafen_psx_hw".to_owned(),
            executable: EmulatorExecutable::Flatpak {
                command: PathBuf::from("/usr/bin/flatpak"),
                app_id: "org.libretro.RetroArch".to_owned(),
            },
            core_path: Some(temp.path().join("mednafen_psx_hw_libretro.so")),
            recommended: true,
        };

        let plan = build_rom_launch_plan(&playlist, "Sony PlayStation", &option).unwrap();
        let arguments = plan
            .arguments
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(arguments.contains(&format!("--filesystem={}", playlist_directory.display())));
        assert!(arguments.contains(&format!("--filesystem={}", disc_directory.display())));
        assert!(arguments.contains(&"org.libretro.RetroArch".to_owned()));
    }

    #[test]
    fn mame_arcade_plan_uses_rompath_and_set_name_without_extracting_archive() {
        let temp = TempDir::new().unwrap();
        let rom = temp.path().join("pong.zip");
        fs::write(&rom, b"romset").unwrap();
        let option = RomEmulatorOption {
            emulator_id: "mame-id".to_owned(),
            emulator_name: "MAME".to_owned(),
            runtime_kind: EmulatorRuntimeKind::Standalone,
            core_name: String::new(),
            executable: EmulatorExecutable::Flatpak {
                command: PathBuf::from("/usr/bin/flatpak"),
                app_id: "org.mamedev.MAME".to_owned(),
            },
            core_path: None,
            recommended: true,
        };

        let plan = build_rom_launch_plan(&rom, "Arcade Laserdisc", &option).unwrap();
        assert_eq!(plan.program, Path::new("/usr/bin/flatpak"));
        assert_eq!(plan.arguments[2], "org.mamedev.MAME");
        assert_eq!(plan.arguments[3], "-rompath");
        assert!(
            plan.arguments[4]
                .to_string_lossy()
                .contains(&temp.path().to_string_lossy().to_string())
        );
        assert_eq!(plan.arguments[5], "pong");
        assert!(
            !plan
                .arguments
                .iter()
                .any(|argument| argument == rom.as_os_str())
        );
    }

    #[test]
    fn hypseus_plan_validates_bundle_and_support_tree() {
        let temp = TempDir::new().unwrap();
        let support = temp.path().join("hypseus");
        let bundle = temp.path().join("Laserdisc Collection/Hypseus Singe");
        let framefile = bundle.join("vldp/lair/lair.txt");
        let rom_directory = bundle.join("roms");
        fs::create_dir_all(support.join("pics")).unwrap();
        fs::create_dir_all(support.join("fonts")).unwrap();
        fs::create_dir_all(framefile.parent().unwrap()).unwrap();
        fs::create_dir_all(&rom_directory).unwrap();
        fs::write(support.join("pics/overlayleds2.bmp"), b"leds").unwrap();
        fs::write(support.join("fonts/default.ttf"), b"font").unwrap();
        fs::write(support.join("hypinput.ini"), b"input").unwrap();
        fs::write(&framefile, b"video.m2v\n").unwrap();
        let option = RomEmulatorOption {
            emulator_id: "hypseus-id".to_owned(),
            emulator_name: "Hypseus Singe".to_owned(),
            runtime_kind: EmulatorRuntimeKind::Standalone,
            core_name: String::new(),
            executable: EmulatorExecutable::Native(support.join("hypseus")),
            core_path: None,
            recommended: true,
        };

        let plan = build_rom_launch_plan(&framefile, "Arcade Laserdisc", &option).unwrap();
        assert_eq!(plan.current_directory, support);
        assert_eq!(plan.arguments[0], "lair");
        assert_eq!(plan.arguments[1], "vldp");
        assert!(
            plan.arguments
                .iter()
                .any(|argument| argument == "-framefile")
        );
        assert!(
            plan.arguments
                .iter()
                .any(|argument| argument == framefile.as_os_str())
        );
        assert!(plan.arguments.iter().any(|argument| argument == "-romdir"));
        assert!(plan.arguments.iter().any(|argument| {
            argument
                .to_string_lossy()
                .contains(&rom_directory.to_string_lossy().to_string())
        }));
    }

    #[test]
    fn known_arcade_cli_emulator_receives_the_archive_path_directly() {
        let temp = TempDir::new().unwrap();
        let rom = temp.path().join("game.zip");
        fs::write(&rom, b"rom").unwrap();
        let option = RomEmulatorOption {
            emulator_id: "supermodel-id".to_owned(),
            emulator_name: "Supermodel".to_owned(),
            runtime_kind: EmulatorRuntimeKind::Standalone,
            core_name: String::new(),
            executable: EmulatorExecutable::Native(PathBuf::from("/usr/bin/supermodel")),
            core_path: None,
            recommended: true,
        };

        let plan = build_rom_launch_plan(&rom, "Arcade", &option).unwrap();
        assert_eq!(plan.arguments, [rom.into_os_string()]);
    }

    #[test]
    fn arcade_emulator_without_a_safe_profile_is_rejected() {
        let temp = TempDir::new().unwrap();
        let rom = temp.path().join("game.zip");
        fs::write(&rom, b"rom").unwrap();
        let option = RomEmulatorOption {
            emulator_id: "teknoparrot-id".to_owned(),
            emulator_name: "TeknoParrot".to_owned(),
            runtime_kind: EmulatorRuntimeKind::Standalone,
            core_name: String::new(),
            executable: EmulatorExecutable::Native(PathBuf::from("/usr/bin/teknoparrot")),
            core_path: None,
            recommended: true,
        };

        let error = build_rom_launch_plan(&rom, "Arcade", &option).unwrap_err();
        assert!(error.to_string().contains("safe TeknoParrot"));
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
        let plan = build_plan_for_choice(
            &prepared,
            kind,
            choice,
            &crate::settings::ResolvedLaunchCustomization::default(),
        )
        .unwrap();
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

    #[test]
    fn wine_launch_maps_only_absolute_paths_and_sets_an_isolated_prefix() {
        let executable = EmulatorExecutable::Wine {
            command: PathBuf::from("/usr/bin/wine"),
            executable: PathBuf::from("/data/Altirra64.exe"),
            prefix: PathBuf::from("/data/wine-prefixes/altirra"),
        };

        assert_eq!(
            path_argument_for_executable(Path::new("/roms/Atari 800/game.atr"), &executable),
            OsString::from(r"Z:\roms\Atari 800\game.atr")
        );
        assert_eq!(
            path_argument_for_executable(Path::new("relative/game.atr"), &executable),
            OsString::from("relative/game.atr")
        );
        assert_eq!(
            launch_environment(&executable),
            vec![(
                OsString::from("WINEPREFIX"),
                OsString::from("/data/wine-prefixes/altirra")
            )]
        );
    }
}
