//! Native macOS/Windows RetroArch profile configuration without physical mapping.
//!
//! `frontend_autoconfig` deliberately means that RetroArch keeps ownership of its
//! host input driver, autoconfiguration database, manual bindings, and device
//! enumeration. This module only prepares a private append configuration for a
//! reviewed core/device topology and a private core-options snapshot.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

pub const NESTOPIA_NES_TWO_PLAYER_PROFILE: &str = "retroarch:nestopia:nes-2player";
pub const NESTOPIA_NES_FOUR_PLAYER_PROFILE: &str = "retroarch:nestopia:nes-4player";
const MAX_CAPTURED_CONFIG_BYTES: u64 = 8 * 1024 * 1024;
pub const WINDOWS_RETROARCH_1_19_1_SHA256: &str =
    "738ca659d2360cedbc62bab7b53c6e9bb20c7d92dfe3de743fa4f3b1fa218e7b";
pub const WINDOWS_NESTOPIA_SHA256: &str =
    "58445c86e4f1858bbe5a68eb4f7b120419f1418a1a9f7743d81076321fafa296";
pub const MACOS_RETROARCH_1_22_2_SHA256: &str =
    "ed90b54434a2899de0ddbfed59f335255fb462c8691bd970a1a761ebb5656d65";
pub const MACOS_NESTOPIA_SHA256: &str =
    "31bdba996461c5214e706ca1c674138976f6932af0a4da1de431b031cf706540";

/// One immutable file from the audited official RetroArch Windows package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuditedRuntimeFile {
    pub relative_path: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}

/// Complete executable/runtime-file closure extracted from the official
/// RetroArch 1.19.1 Windows x86_64 distribution. Runtime-created configuration,
/// assets, cores, logs, saves, and states may coexist beside these files; every
/// entry listed here must have the exact audited length and digest.
pub const WINDOWS_RETROARCH_1_19_1_DISTRIBUTION: &[AuditedRuntimeFile] = &[
    AuditedRuntimeFile {
        relative_path: "avcodec-58.dll",
        bytes: 15859200,
        sha256: "479b7f81c48408f08a107767c989e945b215754bcb9fb239843a88087d0a303a",
    },
    AuditedRuntimeFile {
        relative_path: "avformat-58.dll",
        bytes: 2285568,
        sha256: "55b1d65b5fbae44c980576a48a6fee742af7e3450adba5c4d76ae2d6c2a758dc",
    },
    AuditedRuntimeFile {
        relative_path: "avutil-56.dll",
        bytes: 564224,
        sha256: "694b0c0441ea63ac0ff8af2198b802728f2ecedd97ac879285306ddd43d1905e",
    },
    AuditedRuntimeFile {
        relative_path: "libass-9.dll",
        bytes: 1735422,
        sha256: "48284eb1f40c0c9bc102918f9ff2dad70f02e49c78a43f527749ca04aa56d336",
    },
    AuditedRuntimeFile {
        relative_path: "libbluray-1.dll",
        bytes: 1560887,
        sha256: "dff56a74ea2739047850f98154e738693f3279cf59d418043fa0a2a4ad88a949",
    },
    AuditedRuntimeFile {
        relative_path: "libbz2.dll",
        bytes: 519523,
        sha256: "014a65c908ee5ccb39ef547a0c1a97558191d5d6b175ad85f0bca4d8e733a1d0",
    },
    AuditedRuntimeFile {
        relative_path: "libcrypto-1_1-x64.dll",
        bytes: 3660539,
        sha256: "6774e2f003d21a8e1b64a384d0fc4149df383126a95e1be23b50cddf882c2049",
    },
    AuditedRuntimeFile {
        relative_path: "libexpat-1.dll",
        bytes: 865567,
        sha256: "a7c5ae14e6a1a5393ec359f215547a5bca15550959cc54fe6b3b4f2651474ef6",
    },
    AuditedRuntimeFile {
        relative_path: "libfontconfig-1.dll",
        bytes: 1598311,
        sha256: "68c2b8bb2a597bcf4cb7a0a99e3f3be217e7b765ee837da9cf740fa656957e23",
    },
    AuditedRuntimeFile {
        relative_path: "libfreetype-6.dll",
        bytes: 4412297,
        sha256: "1e073b17f3731530c8f8eeb83d99f247e32f942acadde8c8c3e754ae005a920e",
    },
    AuditedRuntimeFile {
        relative_path: "libfribidi-0.dll",
        bytes: 327911,
        sha256: "3ed6b44da0a3b15e02d7bebdfe6505d8b9755131cf191cf72ad537ddec9c79b1",
    },
    AuditedRuntimeFile {
        relative_path: "libgcc_s_seh-1.dll",
        bytes: 587193,
        sha256: "676a168a575b00bc17aaa6dbda98217f1fa7a49782a51030dcaf8595159bba04",
    },
    AuditedRuntimeFile {
        relative_path: "libglib-2.0-0.dll",
        bytes: 1806015,
        sha256: "87532638315a5c23494ce7eea0ad6f5a40e51dfb04ef8b5d16706cd391d0d53a",
    },
    AuditedRuntimeFile {
        relative_path: "libgmp-10.dll",
        bytes: 1039873,
        sha256: "051538b33ed5d623c660cb4ac01b2764d09c6cefe8502497fdf2ca877509e9f8",
    },
    AuditedRuntimeFile {
        relative_path: "libgnutls-30.dll",
        bytes: 2793913,
        sha256: "39dd473ecc4f98210fe53d266585484386e479df0c58f1a01b8f1954fb10d18d",
    },
    AuditedRuntimeFile {
        relative_path: "libharfbuzz-0.dll",
        bytes: 8025077,
        sha256: "c85c0423fa8a05025318adb89097fc095cc4ccc707ceb586c7d6681bec3956b4",
    },
    AuditedRuntimeFile {
        relative_path: "libhogweed-6.dll",
        bytes: 8589721,
        sha256: "9c45ed13cda45f049aa1e8f6b7c1e71b9af145687a055f3ca2d4ed3598fafa55",
    },
    AuditedRuntimeFile {
        relative_path: "libiconv-2.dll",
        bytes: 1798606,
        sha256: "97de1aa86edc93192a193a802de619c33d4e2ee1bda92a5a7df4f0fa5102b473",
    },
    AuditedRuntimeFile {
        relative_path: "libidn2-0.dll",
        bytes: 571117,
        sha256: "07114ea57ccd717623158389ad439cedd52dc6c8008925c027c17a6958e2376d",
    },
    AuditedRuntimeFile {
        relative_path: "libintl-8.dll",
        bytes: 521311,
        sha256: "d644c5f5cb5fca23d66635f2798ccfb42257e90d1d501afe2569e3a3b03abfeb",
    },
    AuditedRuntimeFile {
        relative_path: "liblzma-5.dll",
        bytes: 1004090,
        sha256: "a46cded001c15421190b35484c62fc96a2775a0ef6962dc8b47a2e4bfa9b82bd",
    },
    AuditedRuntimeFile {
        relative_path: "libmp3lame-0.dll",
        bytes: 725680,
        sha256: "13a49e362b804d97f04411654ab325b049cd8343b794f323e72f6b3e6816ae89",
    },
    AuditedRuntimeFile {
        relative_path: "libnettle-8.dll",
        bytes: 7664880,
        sha256: "3860a0898a13f0fdfbcbcb54fa5b92c39df93a3f87dcdbd2d84b79d3d495692d",
    },
    AuditedRuntimeFile {
        relative_path: "libogg-0.dll",
        bytes: 167605,
        sha256: "07fc454eb93c1b5aef1f1cb8e66b6eb0ea6660d695fe10c86e71e3dd46272a3b",
    },
    AuditedRuntimeFile {
        relative_path: "libopencore-amrnb-0.dll",
        bytes: 1276775,
        sha256: "ddad825dfa48970a546dffb6a918a82570526aae62af314ed2a6b1cd7fd384a9",
    },
    AuditedRuntimeFile {
        relative_path: "libopencore-amrwb-0.dll",
        bytes: 553293,
        sha256: "db99c219b86740fb2d76462ae62559b8873134aecd8a927771152bb3b4d18773",
    },
    AuditedRuntimeFile {
        relative_path: "libopus-0.dll",
        bytes: 2475844,
        sha256: "5f45db5e636654a5e59b797652a75b7df3e143b47ac621cc4240f8da159154a6",
    },
    AuditedRuntimeFile {
        relative_path: "libpcre-1.dll",
        bytes: 968001,
        sha256: "f206e6555d364497c6b931421137c4a75363948b06636c936f5111253b92a45b",
    },
    AuditedRuntimeFile {
        relative_path: "libpcre2-16-0.dll",
        bytes: 444309,
        sha256: "9b6abbf7a56535033cd9072566685e09113a6140c6109b335b0b5967903f3ce8",
    },
    AuditedRuntimeFile {
        relative_path: "libpng16-16.dll",
        bytes: 1443144,
        sha256: "d4847543823a0b926ac17af7d5b3a6d11da643e5f7f07e2f834dc123386cddcf",
    },
    AuditedRuntimeFile {
        relative_path: "libsamplerate-0.dll",
        bytes: 1660294,
        sha256: "91d309b9b76266107e645fbc517be153a81a5cbce78cd54f3039b0a983c51dae",
    },
    AuditedRuntimeFile {
        relative_path: "libspeex-1.dll",
        bytes: 757553,
        sha256: "605bb73daf13447d5ea8bd8e0c97baf5f66fb9c9db75bb628f4ce83a83424ebc",
    },
    AuditedRuntimeFile {
        relative_path: "libssl-1_1-x64.dll",
        bytes: 914463,
        sha256: "2722cb82036b75b1d190a2828f5aa9384c9d6a4f55307a86e4c9b9e070f17dc4",
    },
    AuditedRuntimeFile {
        relative_path: "libssp-0.dll",
        bytes: 16941,
        sha256: "718cac645013599465f48ffabff8b3b4f79ff606a8445444bd5fe1254c2ed3b4",
    },
    AuditedRuntimeFile {
        relative_path: "libstdc++-6.dll",
        bytes: 1687137,
        sha256: "9ca015df1af7309dee0153f40cc5ca6656be91c557469f74cdff5d06e03fa9cb",
    },
    AuditedRuntimeFile {
        relative_path: "libtasn1-6.dll",
        bytes: 552745,
        sha256: "299bf36b9f17a99934f32584a63b991ac6814e02da6387bc3c1e19f705c5aeca",
    },
    AuditedRuntimeFile {
        relative_path: "libtheoradec-1.dll",
        bytes: 219367,
        sha256: "e000ee681205686c831ddbcec124c3b29e9c86a36b76518f520653232443aa21",
    },
    AuditedRuntimeFile {
        relative_path: "libunistring-2.dll",
        bytes: 5209890,
        sha256: "3765b9a6db9db00d85279110533f08365d2d6a975a36eb9ae5dddd33450ed825",
    },
    AuditedRuntimeFile {
        relative_path: "libvo-amrwbenc-0.dll",
        bytes: 837415,
        sha256: "add97ae252ff3cf0068800706c33a848d09aab74f326416bb210abe9d5fd2f2c",
    },
    AuditedRuntimeFile {
        relative_path: "libvorbis-0.dll",
        bytes: 378511,
        sha256: "82377fbe7a5742e7265b9e74e1e91ff9bca56d0cb1281057970028ed1436d3bc",
    },
    AuditedRuntimeFile {
        relative_path: "libvorbisenc-2.dll",
        bytes: 770637,
        sha256: "8b39ee16f4a34589569dd6ae8a8d7e0099406c056b6a4844596de88b6692cbf3",
    },
    AuditedRuntimeFile {
        relative_path: "libwinpthread-1.dll",
        bytes: 58537,
        sha256: "5bdcf101bc2e7397de329b7c42353d07251c85b15ce3f17ea0450cca7e4130d0",
    },
    AuditedRuntimeFile {
        relative_path: "libx264-155.dll",
        bytes: 3126491,
        sha256: "9625c49ec494e2a63b32f688eb174c3b48603f4e3c6b53c25d8d16d7555b8f33",
    },
    AuditedRuntimeFile {
        relative_path: "libxml2-2.dll",
        bytes: 6887484,
        sha256: "cf92faa6d201a39efde22a368926e1d2a6dd5c250a63975cd594a649caf63db7",
    },
    AuditedRuntimeFile {
        relative_path: "libzstd.dll",
        bytes: 1049330,
        sha256: "d2c5b754becd62f10e50a98e70094e39eef3ec357b0c947f0ee94ecdeb23e51b",
    },
    AuditedRuntimeFile {
        relative_path: "nvdaControllerClient64.dll",
        bytes: 153600,
        sha256: "41c1f5df5997e798fcfbf7c8f2589de811e768b069a60710600cf57cb23a0b09",
    },
    AuditedRuntimeFile {
        relative_path: "platforms/qwindows.dll",
        bytes: 1063936,
        sha256: "f506e4c95f2cc03f9c8b3a340c12020f139de2c4886798c997b1ef9b8fea1e50",
    },
    AuditedRuntimeFile {
        relative_path: "qt.conf",
        bytes: 21,
        sha256: "87a797e45892b4bbdf97054ae08a4a3ad43b407395110530c46c20c8787b9596",
    },
    AuditedRuntimeFile {
        relative_path: "Qt5Core.dll",
        bytes: 6005248,
        sha256: "1a8a3aa2a97dcce05a7f49890a5dabad042f1e845505bd14053661377ae42429",
    },
    AuditedRuntimeFile {
        relative_path: "Qt5Gui.dll",
        bytes: 6536704,
        sha256: "b8331ebae5244c46c871f73c8fb294534bd986a485b4a618d76f26dd21c4b10b",
    },
    AuditedRuntimeFile {
        relative_path: "Qt5Network.dll",
        bytes: 1584640,
        sha256: "ed5c55c74de0fea7667e48ff267c556f18e58b29e304fc09bf076d874eb2f360",
    },
    AuditedRuntimeFile {
        relative_path: "Qt5Widgets.dll",
        bytes: 6041600,
        sha256: "3fe0d578895b1331b7b11d6d88b7fbfec29e1725749a17568c3c6517f4a95045",
    },
    AuditedRuntimeFile {
        relative_path: "retroarch.default.cfg",
        bytes: 34183,
        sha256: "6e0a2a939cc0a83883b4786b8fad0436616392efbbad29fe5309575ced25d423",
    },
    AuditedRuntimeFile {
        relative_path: "retroarch.exe",
        bytes: 16128258,
        sha256: "738ca659d2360cedbc62bab7b53c6e9bb20c7d92dfe3de743fa4f3b1fa218e7b",
    },
    AuditedRuntimeFile {
        relative_path: "SDL2.dll",
        bytes: 10511307,
        sha256: "ceee2e80b0c4c52cc55d9287ba15d046f01c3979516adecdc365609820ab447b",
    },
    AuditedRuntimeFile {
        relative_path: "swresample-3.dll",
        bytes: 122368,
        sha256: "5da8c3f74d97fba1a182ff6f55fb7b9bd35c2a1dcf16063928005aea1f4b4130",
    },
    AuditedRuntimeFile {
        relative_path: "swscale-5.dll",
        bytes: 538624,
        sha256: "c15ef34a4a473b597f9f31f75ba31f9832e3a64c90f9ba6dbc5523058a25d74d",
    },
    AuditedRuntimeFile {
        relative_path: "xvidcore.dll",
        bytes: 1233625,
        sha256: "f8ec09f12cad97c89199c0c7d72e7d0907a707f350ea5d51c3bc114d9a2d50f3",
    },
    AuditedRuntimeFile {
        relative_path: "zlib1.dll",
        bytes: 131072,
        sha256: "5ef0a698331cd7102ca5606983938a5a6a6bab218c89a10216462222104eaff5",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontendAutoconfigHost {
    MacOs,
    Windows,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MacOsInstallDisposition {
    Standard,
    /// The caller has resolved RetroArch's portable marker/Info.plist policy to
    /// this exact directory. Guessing an app-bundle ancestor is not sufficient.
    Portable {
        application_root: PathBuf,
    },
    /// Portable markers or the effective app bundle could not be resolved.
    Ambiguous,
}

/// Apply the same portable decision used by native macOS RetroArch without
/// executing the frontend or modifying its bundle. A malformed/binary plist is
/// intentionally ambiguous when no adjacent marker settles the decision.
pub fn resolve_macos_install_disposition(
    application_bundle: &Path,
) -> Result<MacOsInstallDisposition> {
    ensure_absolute(application_bundle, "RetroArch application bundle")?;
    let application_root = application_bundle
        .parent()
        .context("Native macOS RetroArch application bundle has no containing directory")?;
    let inputs = vec![
        CapturedConfig::capture_optional(&application_root.join("portable.txt"))?,
        CapturedConfig::capture_optional(&application_bundle.join("Contents/Info.plist"))?,
    ];
    macos_install_disposition_from_captures(application_bundle, &inputs)
}

fn macos_install_disposition_from_captures(
    application_bundle: &Path,
    inputs: &[CapturedConfig],
) -> Result<MacOsInstallDisposition> {
    let application_root = application_bundle
        .parent()
        .context("Native macOS RetroArch application bundle has no containing directory")?;
    ensure!(inputs.len() == 2, "Missing macOS path-selection captures");
    if inputs[0].resolved_path.is_some() {
        return Ok(MacOsInstallDisposition::Portable {
            application_root: application_root.to_path_buf(),
        });
    }
    if inputs[1].resolved_path.is_none() {
        return Ok(MacOsInstallDisposition::Ambiguous);
    }
    let bytes = &inputs[1].contents;
    if !bytes.starts_with(b"<?xml") && !bytes.starts_with(b"<plist") {
        return Ok(MacOsInstallDisposition::Ambiguous);
    }
    let mut reader = quick_xml::Reader::from_reader(bytes.as_slice());
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut portable = None;
    let mut awaiting_value = false;
    loop {
        use quick_xml::events::Event;
        match reader.read_event_into(&mut buffer)? {
            Event::Start(element) if element.name().as_ref() == b"key" => {
                let key = reader.read_text(quick_xml::name::QName(b"key"))?;
                awaiting_value = key.as_ref() == "RAPortableInstall";
            }
            Event::Empty(element)
                if awaiting_value && matches!(element.name().as_ref(), b"true" | b"false") =>
            {
                ensure!(portable.is_none(), "Duplicate RAPortableInstall key");
                portable = Some(element.name().as_ref() == b"true");
                awaiting_value = false;
            }
            Event::Start(element)
                if awaiting_value && matches!(element.name().as_ref(), b"true" | b"false") =>
            {
                ensure!(portable.is_none(), "Duplicate RAPortableInstall key");
                portable = Some(element.name().as_ref() == b"true");
                awaiting_value = false;
            }
            Event::Eof => break,
            Event::Text(text) if awaiting_value && text.iter().all(u8::is_ascii_whitespace) => {}
            Event::Comment(_) => {}
            _ if awaiting_value => {
                bail!("RAPortableInstall is not a plist boolean")
            }
            _ => {}
        }
        buffer.clear();
    }
    Ok(if portable.unwrap_or(false) {
        MacOsInstallDisposition::Portable {
            application_root: application_root.to_path_buf(),
        }
    } else {
        MacOsInstallDisposition::Standard
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeRetroArchPathRequest {
    MacOs {
        executable: PathBuf,
        /// Exact main RetroArch app-bundle root. Native `:/` paths expand from
        /// the CFBundle URL (the `.app` root), while portable application data
        /// lives beside the bundle.
        application_bundle: PathBuf,
        home: PathBuf,
        application_support: PathBuf,
        install: MacOsInstallDisposition,
    },
    Windows {
        executable: PathBuf,
        /// Exact inherited HOME used by RetroArch's Win32 `~` expansion. This
        /// is intentionally not inferred from USERPROFILE.
        home: Option<PathBuf>,
        roaming_app_data: PathBuf,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeRetroArchPaths {
    pub host: FrontendAutoconfigHost,
    /// Root used by RetroArch's leading-`:` paths. On macOS this is the
    /// CFBundle URL (`<bundle>`); on Windows it is the executable directory.
    pub application_directory: PathBuf,
    /// Effective native application-data root. On standard macOS installs this
    /// is `~/Library/Application Support/RetroArch`; for portable installs it
    /// is the directory containing the `.app` bundle. On Windows it is the
    /// directory containing the selected main configuration.
    pub application_data_directory: PathBuf,
    /// The effective main configuration selected by the native frontend.
    pub main_config: PathBuf,
    /// Default menu-config/per-core-option directory before user overrides.
    pub default_config_directory: PathBuf,
    pub home: Option<PathBuf>,
}

impl NativeRetroArchPaths {
    pub fn default_config_directory(&self) -> PathBuf {
        self.default_config_directory.clone()
    }

    pub fn default_core_options_path(&self) -> PathBuf {
        // RetroArch resolves the global core-options filename relative to the
        // selected RARCH_PATH_CONFIG. This differs from application_directory
        // when Windows falls back to %APPDATA% and on macOS, where retroarch.cfg
        // is stored in the application-data root's config subdirectory.
        self.main_config
            .with_file_name("retroarch-core-options.cfg")
    }

    pub fn resolve_configured_path(&self, value: &str) -> Result<PathBuf> {
        ensure!(
            !value.is_empty() && !value.chars().any(char::is_control),
            "frontend_autoconfig path is empty or contains a control character"
        );
        if let Some(relative) = value
            .strip_prefix("~/")
            .or_else(|| value.strip_prefix("~\\"))
        {
            ensure!(
                !relative.is_empty(),
                "frontend_autoconfig home path is empty"
            );
            return Ok(self
                .home
                .as_ref()
                .context("RetroArch HOME is unavailable for a configured '~' path")?
                .join(relative));
        }
        if let Some(relative) = value
            .strip_prefix(":/")
            .or_else(|| value.strip_prefix(":\\"))
        {
            ensure!(
                !relative.is_empty(),
                "frontend_autoconfig application-relative path is empty"
            );
            return Ok(self.application_directory.join(relative));
        }
        let path = PathBuf::from(value);
        if path.is_absolute()
            || (self.host == FrontendAutoconfigHost::Windows
                && is_lexically_absolute_windows_path(value))
        {
            return Ok(path);
        }
        bail!(
            "Relative RetroArch path needs effective frontend resolution before frontend_autoconfig"
        )
    }
}

fn is_lexically_absolute_windows_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\'))
        || value.starts_with("\\\\")
        || value.starts_with("//")
}

pub fn resolve_native_retroarch_paths(
    request: &NativeRetroArchPathRequest,
    can_load_config: impl Fn(&Path) -> bool,
) -> Result<NativeRetroArchPaths> {
    match request {
        NativeRetroArchPathRequest::MacOs {
            executable,
            application_bundle,
            home,
            application_support,
            install,
        } => {
            ensure_absolute(executable, "RetroArch executable")?;
            ensure_absolute(application_bundle, "RetroArch application bundle")?;
            ensure_absolute(home, "home directory")?;
            ensure_absolute(application_support, "Application Support directory")?;
            ensure!(
                application_bundle.extension() == Some(OsStr::new("app")),
                "Native macOS RetroArch application bundle must have an .app extension"
            );
            let executable_directory = application_bundle.join("Contents/MacOS");
            ensure!(
                executable.parent() == Some(executable_directory.as_path()),
                "Native macOS RetroArch executable must be directly under <bundle>/Contents/MacOS"
            );
            let application_container = application_bundle
                .parent()
                .context("Native macOS RetroArch application bundle has no containing directory")?;
            let application_data = match install {
                MacOsInstallDisposition::Standard => application_support.join("RetroArch"),
                MacOsInstallDisposition::Portable { application_root } => {
                    ensure_absolute(application_root, "portable RetroArch root")?;
                    ensure!(
                        application_root == application_container,
                        "Portable RetroArch root must be the directory containing the native application bundle"
                    );
                    application_root.clone()
                }
                MacOsInstallDisposition::Ambiguous => bail!(
                    "Ambiguous macOS RetroArch portable layout; frontend_autoconfig requires an exact application-data root"
                ),
            };
            Ok(NativeRetroArchPaths {
                host: FrontendAutoconfigHost::MacOs,
                application_directory: application_bundle.clone(),
                application_data_directory: application_data.clone(),
                main_config: application_data.join("config/retroarch.cfg"),
                default_config_directory: application_data.join("config"),
                home: Some(home.clone()),
            })
        }
        NativeRetroArchPathRequest::Windows {
            executable,
            home,
            roaming_app_data,
        } => {
            ensure_absolute(executable, "RetroArch executable")?;
            if let Some(home) = home {
                ensure_absolute(home, "home directory")?;
            }
            ensure_absolute(roaming_app_data, "roaming application-data directory")?;
            let application_directory = executable
                .parent()
                .context("Native Windows RetroArch executable has no parent directory")?
                .to_path_buf();
            let local = application_directory.join("retroarch.cfg");
            let fallback = roaming_app_data.join("retroarch.cfg");
            // Native RetroArch checks beside retroarch.exe first. If neither file
            // exists, that same location is its first-run creation target.
            // Upstream tests whether each candidate can be loaded, rather than
            // checking existence alone. The integration closure must use the
            // same read/parser policy that will supply the captured config.
            let main_config = if can_load_config(&local) || !can_load_config(&fallback) {
                local
            } else {
                fallback
            };
            Ok(NativeRetroArchPaths {
                host: FrontendAutoconfigHost::Windows,
                default_config_directory: application_directory.join("config"),
                application_data_directory: main_config
                    .parent()
                    .context("Native Windows RetroArch config has no parent directory")?
                    .to_path_buf(),
                application_directory,
                main_config,
                home: home.clone(),
            })
        }
    }
}

fn ensure_absolute(path: &Path, name: &str) -> Result<()> {
    ensure!(
        path.is_absolute(),
        "frontend_autoconfig {name} must be absolute"
    );
    Ok(())
}

pub fn validate_unmodified_launch_context(
    arguments: &[OsString],
    environment: &[(OsString, OsString)],
) -> Result<()> {
    ensure!(
        environment.is_empty(),
        "Custom launch environment needs exact native RetroArch path resolution before frontend_autoconfig"
    );
    let mut options = true;
    for argument in arguments {
        let argument = argument
            .to_str()
            .context("Non-UTF-8 RetroArch argument needs exact configuration resolution")?;
        if options && argument == "--" {
            options = false;
            continue;
        }
        if options
            && (matches!(argument, "--config" | "-c" | "--appendconfig")
                || argument.starts_with("--config=")
                || (argument.starts_with("-c") && argument.len() > 2)
                || argument.starts_with("--appendconfig="))
        {
            bail!(
                "Custom RetroArch config/appendconfig arguments need effective resolution before frontend_autoconfig"
            );
        }
        if options
            && (matches!(
                argument,
                "--device" | "-d" | "--nodevice" | "-N" | "--dualanalog" | "-A"
            ) || [
                "--device=",
                "--nodevice=",
                "--dualanalog=",
                "-d",
                "-N",
                "-A",
            ]
            .iter()
            .any(|prefix| argument.starts_with(prefix) && argument.len() > prefix.len()))
        {
            bail!(
                "RetroArch command-line device selection conflicts with frontend_autoconfig topology"
            );
        }
    }
    Ok(())
}

pub fn attach_frontend_autoconfig(
    host: FrontendAutoconfigHost,
    arguments: &[OsString],
    environment: &[(OsString, OsString)],
    append_config: &Path,
    port_modes: &[FrontendPortMode],
) -> Result<Vec<OsString>> {
    validate_unmodified_launch_context(arguments, environment)?;
    ensure_absolute(append_config, "append-config path")?;
    let encoded = encode_config_path(host, append_config)?;
    ensure!(
        !encoded.contains('|'),
        "frontend_autoconfig append-config path contains RetroArch's list separator"
    );
    let mut result = Vec::with_capacity(arguments.len() + 2 + port_modes.len());
    result.push(OsString::from("--appendconfig"));
    result.push(OsString::from(encoded));
    for mode in port_modes {
        result.push(OsString::from(format!(
            "--device={}:{}",
            mode.frontend_port, mode.libretro_device
        )));
    }
    result.extend_from_slice(arguments);
    Ok(result)
}

pub fn resolve_effective_core_options_path(
    paths: &NativeRetroArchPaths,
    main_config: &str,
    library: &str,
    content: &Path,
    is_file: impl Fn(&Path) -> bool,
) -> Result<PathBuf> {
    let (precedence, fallback) =
        effective_core_options_candidates(paths, main_config, library, content)?;
    Ok(precedence
        .into_iter()
        .find(|path| is_file(path))
        .unwrap_or(fallback))
}

fn effective_core_options_candidates(
    paths: &NativeRetroArchPaths,
    main_config: &str,
    library: &str,
    content: &Path,
) -> Result<(Vec<PathBuf>, PathBuf)> {
    reject_includes(main_config, "main RetroArch configuration")?;
    ensure!(
        !library.is_empty()
            && !library.contains(['/', '\\'])
            && !library.chars().any(char::is_control),
        "Invalid RetroArch library name for frontend_autoconfig"
    );
    let config_directory = match config_value(main_config, "rgui_config_directory")? {
        Some(value) if value.is_empty() || value == "default" => paths
            .main_config
            .parent()
            .context("frontend_autoconfig main config has no parent directory")?
            .to_path_buf(),
        Some(value) => paths.resolve_configured_path(&value)?,
        None => paths.default_config_directory(),
    };
    let library_directory = config_directory.join(library);
    let mut candidates = Vec::new();
    if config_bool(main_config, "game_specific_options", true)? {
        let game = content
            .file_stem()
            .context("frontend_autoconfig content has no game basename")?;
        let folder = content
            .parent()
            .and_then(Path::file_name)
            .context("frontend_autoconfig content has no folder basename")?;
        for name in [game, folder] {
            let mut filename = name.to_os_string();
            filename.push(".opt");
            candidates.push(library_directory.join(filename));
        }
    }
    if !config_bool(main_config, "global_core_options", false)? {
        candidates.push(library_directory.join(format!("{library}.opt")));
    }
    let fallback =
        match config_value(main_config, "core_options_path")?.filter(|value| !value.is_empty()) {
            Some(value) => paths.resolve_configured_path(&value),
            None => Ok(paths.default_core_options_path()),
        }?;
    Ok((candidates, fallback))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontendAutoconfigProfileSpec {
    pub id: String,
    pub core: String,
    pub target_layout: String,
    pub transport: String,
    pub retroarch_library: Option<String>,
    pub explicit_selection: bool,
    pub platforms: BTreeSet<String>,
    pub content_extensions: BTreeSet<String>,
    pub frontend_ports: usize,
    pub max_players: usize,
    pub default_device: u32,
    pub port_devices: BTreeMap<usize, u32>,
    pub core_options: BTreeMap<String, String>,
    pub content_guard: Option<String>,
    pub requires_fresh_start: bool,
    pub has_player_topology: bool,
    pub dynamic_profile: bool,
    pub special_preparation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontendPortMode {
    pub frontend_port: usize,
    pub libretro_device: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontendAutoconfigArtifacts {
    pub append_config_path: PathBuf,
    pub append_config: String,
    pub core_options_path: PathBuf,
    pub core_options: String,
    pub port_modes: Vec<FrontendPortMode>,
    pub description: String,
}

/// Retains the private configuration for the entire child lifetime.
pub struct FrontendAutoconfigSession {
    directory: tempfile::TempDir,
    request: NativeRetroArchPathRequest,
    verify_macos_bundle_seal: bool,
    path_selection_inputs: Vec<CapturedConfig>,
    core_options_selection_inputs: Vec<CapturedConfig>,
    pinned_frontend: Option<CapturedFileIdentity>,
    pinned_core: Option<CapturedFileIdentity>,
    pinned_runtime_files: Vec<CapturedFileIdentity>,
    pinned_content: Option<CapturedFileIdentity>,
    prepared_arguments: Option<Vec<OsString>>,
    prepared_environment: Option<Vec<(OsString, OsString)>>,
    main_config: CapturedConfig,
    effective_core_options: CapturedConfig,
    private_append_config: CapturedConfig,
    private_core_options: CapturedConfig,
    private_directories: Vec<CapturedConfig>,
    pub paths: NativeRetroArchPaths,
    pub effective_core_options_path: PathBuf,
    pub artifacts: FrontendAutoconfigArtifacts,
}

impl FrontendAutoconfigSession {
    pub fn root(&self) -> &Path {
        self.directory.path()
    }

    pub fn verify(&self) -> Result<()> {
        #[cfg(target_os = "macos")]
        if self.verify_macos_bundle_seal {
            let NativeRetroArchPathRequest::MacOs {
                application_bundle, ..
            } = &self.request
            else {
                bail!("macOS bundle-seal verification was attached to a non-macOS request");
            };
            verify_macos_bundle_seal(application_bundle)?;
        }
        let current_inputs = path_selection_inputs(&self.request)?;
        if let NativeRetroArchPathRequest::MacOs {
            application_bundle,
            install,
            ..
        } = &self.request
        {
            ensure!(
                &macos_install_disposition_from_captures(application_bundle, &current_inputs)?
                    == install,
                "Native macOS RetroArch portable disposition changed after preparation"
            );
        }
        let current_candidate_loadable = |path: &Path| {
            current_inputs
                .iter()
                .find(|capture| capture.requested_path == path)
                .is_some_and(|capture| capture.loadable)
        };
        let current_paths =
            resolve_native_retroarch_paths(&self.request, current_candidate_loadable)?;
        ensure!(
            current_paths == self.paths,
            "Native RetroArch configuration path selection changed after preparation"
        );
        for capture in [
            &self.main_config,
            &self.effective_core_options,
            &self.private_append_config,
            &self.private_core_options,
        ] {
            capture.verify()?;
        }
        for directory in &self.private_directories {
            directory.verify()?;
        }
        ensure!(
            current_inputs == self.path_selection_inputs,
            "Native RetroArch path-selection inputs changed after preparation"
        );
        for capture in &self.core_options_selection_inputs {
            capture.verify()?;
        }
        if let Some(frontend) = &self.pinned_frontend {
            frontend.verify()?;
        }
        if let Some(core) = &self.pinned_core {
            core.verify()?;
        }
        for runtime_file in &self.pinned_runtime_files {
            runtime_file.verify()?;
        }
        if !self.pinned_runtime_files.is_empty()
            && let NativeRetroArchPathRequest::Windows { executable, .. } = &self.request
        {
            verify_windows_runtime_loadable_members(executable)?;
        }
        if let Some(content) = &self.pinned_content {
            content.verify()?;
        }
        verify_private_security(
            &self.private_directories,
            &[&self.private_append_config, &self.private_core_options],
        )?;
        ensure_no_physical_mapping_keys(&self.artifacts.append_config)
    }

    pub fn verify_launch_identity(
        &self,
        program: &Path,
        core: &Path,
        content: &Path,
        arguments: &[OsString],
        environment: &[(OsString, OsString)],
    ) -> Result<()> {
        let executable = match &self.request {
            NativeRetroArchPathRequest::MacOs { executable, .. }
            | NativeRetroArchPathRequest::Windows { executable, .. } => executable,
        };
        ensure!(
            program == executable,
            "Native RetroArch program changed after preparation"
        );
        ensure!(
            self.prepared_arguments.as_deref() == Some(arguments),
            "Native RetroArch arguments changed after frontend_autoconfig preparation"
        );
        ensure!(
            self.prepared_environment.as_deref() == Some(environment),
            "Native RetroArch environment changed after frontend_autoconfig preparation"
        );
        ensure!(
            self.pinned_frontend
                .as_ref()
                .is_some_and(|capture| capture.requested_path == program),
            "Native RetroArch frontend changed after frontend_autoconfig preparation"
        );
        ensure!(
            self.pinned_core
                .as_ref()
                .is_some_and(|capture| capture.requested_path == core),
            "Native Nestopia core changed after frontend_autoconfig preparation"
        );
        ensure!(
            self.pinned_content
                .as_ref()
                .is_some_and(|capture| capture.requested_path == content),
            "Native RetroArch content changed after frontend_autoconfig preparation"
        );
        self.verify()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapturedFileIdentity {
    requested_path: PathBuf,
    resolved_path: PathBuf,
    bytes: u64,
    sha256: String,
}

impl CapturedFileIdentity {
    fn capture(path: &Path) -> Result<Self> {
        use sha2::{Digest, Sha256};

        let resolved_path = std::fs::canonicalize(path)
            .with_context(|| format!("Resolving pinned binary {}", path.display()))?;
        let mut file = std::fs::File::open(path)
            .with_context(|| format!("Opening pinned binary {}", path.display()))?;
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file(),
            "Pinned binary is not a regular file: {}",
            path.display()
        );
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        ensure!(
            std::fs::canonicalize(path)? == resolved_path,
            "Pinned binary changed identity while it was captured: {}",
            path.display()
        );
        Ok(Self {
            requested_path: path.to_path_buf(),
            resolved_path,
            bytes: metadata.len(),
            sha256: hex::encode(digest.finalize()),
        })
    }

    fn capture_expected(path: &Path, expected: &AuditedRuntimeFile) -> Result<Self> {
        let capture = Self::capture(path)?;
        ensure!(
            capture.bytes == expected.bytes,
            "Audited RetroArch runtime file has unexpected length: {}",
            path.display()
        );
        ensure!(
            capture.sha256 == expected.sha256,
            "Audited RetroArch runtime file has unexpected digest: {}",
            path.display()
        );
        Ok(capture)
    }

    fn verify(&self) -> Result<()> {
        ensure!(
            Self::capture(&self.requested_path)? == *self,
            "Pinned binary changed after frontend_autoconfig preparation: {}",
            self.requested_path.display()
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapturedConfig {
    requested_path: PathBuf,
    resolved_path: Option<PathBuf>,
    contents: Vec<u8>,
    path_valid: bool,
    loadable: bool,
    fingerprint: Option<CapturedMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapturedMetadata {
    is_file: bool,
    is_dir: bool,
    len: u64,
    readonly: bool,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    mode: u32,
    #[cfg(unix)]
    uid: u32,
    #[cfg(unix)]
    gid: u32,
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
}

impl CapturedMetadata {
    fn from_path(path: &Path) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(path)
            .or_else(|_| std::fs::symlink_metadata(path))
            .map(|metadata| Self {
                is_file: metadata.is_file(),
                is_dir: metadata.is_dir(),
                len: metadata.len(),
                readonly: metadata.permissions().readonly(),
                modified: metadata.modified().ok(),
                #[cfg(unix)]
                mode: metadata.mode(),
                #[cfg(unix)]
                uid: metadata.uid(),
                #[cfg(unix)]
                gid: metadata.gid(),
                #[cfg(unix)]
                dev: metadata.dev(),
                #[cfg(unix)]
                ino: metadata.ino(),
            })
            .unwrap_or(Self {
                is_file: false,
                is_dir: false,
                len: 0,
                readonly: false,
                modified: None,
                #[cfg(unix)]
                mode: 0,
                #[cfg(unix)]
                uid: 0,
                #[cfg(unix)]
                gid: 0,
                #[cfg(unix)]
                dev: 0,
                #[cfg(unix)]
                ino: 0,
            })
    }
}

impl CapturedConfig {
    fn capture_optional(path: &Path) -> Result<Self> {
        match std::fs::symlink_metadata(path) {
            Ok(_) => {
                let path_valid = std::fs::metadata(path).is_ok();
                let fingerprint = Some(CapturedMetadata::from_path(path));
                let resolved_path = std::fs::canonicalize(path).ok();
                let mut file = match std::fs::File::open(path) {
                    Ok(file) => file,
                    Err(_) => {
                        return Ok(Self {
                            requested_path: path.to_path_buf(),
                            resolved_path,
                            contents: Vec::new(),
                            path_valid,
                            loadable: false,
                            fingerprint,
                        });
                    }
                };
                let metadata = match file.metadata() {
                    Ok(metadata) if metadata.is_file() => metadata,
                    _ => {
                        return Ok(Self {
                            requested_path: path.to_path_buf(),
                            resolved_path,
                            contents: Vec::new(),
                            path_valid,
                            loadable: false,
                            fingerprint,
                        });
                    }
                };
                ensure!(
                    metadata.len() <= MAX_CAPTURED_CONFIG_BYTES,
                    "{} exceeds the frontend_autoconfig capture limit",
                    path.display()
                );
                let mut contents = Vec::with_capacity(metadata.len() as usize);
                std::io::Read::by_ref(&mut file)
                    .take(MAX_CAPTURED_CONFIG_BYTES + 1)
                    .read_to_end(&mut contents)?;
                ensure!(
                    contents.len() as u64 <= MAX_CAPTURED_CONFIG_BYTES,
                    "{} grew beyond the frontend_autoconfig capture limit",
                    path.display()
                );
                let resolved_after = std::fs::canonicalize(path)
                    .with_context(|| format!("Resolving {} after capture", path.display()))?;
                ensure!(
                    resolved_path.as_ref() == Some(&resolved_after),
                    "{} changed identity while it was captured",
                    path.display()
                );
                Ok(Self {
                    requested_path: path.to_path_buf(),
                    resolved_path,
                    contents,
                    path_valid,
                    loadable: true,
                    fingerprint,
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                requested_path: path.to_path_buf(),
                resolved_path: None,
                contents: Vec::new(),
                path_valid: false,
                loadable: false,
                fingerprint: None,
            }),
            Err(error) => {
                Err(error).with_context(|| format!("Reading metadata for {}", path.display()))
            }
        }
    }

    fn capture_required(path: &Path) -> Result<Self> {
        let captured = Self::capture_optional(path)?;
        ensure!(
            captured.loadable,
            "{} is not a readable regular file",
            path.display()
        );
        Ok(captured)
    }

    fn text(&self) -> Result<&str> {
        std::str::from_utf8(&self.contents)
            .with_context(|| format!("{} is not UTF-8", self.requested_path.display()))
    }

    fn verify(&self) -> Result<()> {
        let current = Self::capture_optional(&self.requested_path)?;
        ensure!(
            current == *self,
            "{} changed after frontend_autoconfig preparation",
            self.requested_path.display()
        );
        Ok(())
    }
}

/// Resolve the native frontend's real configuration inputs, copy only the
/// effective core options into a verified private snapshot, and return argv
/// with the private append file applied before the original arguments.
pub fn prepare_frontend_autoconfig_session(
    request: &NativeRetroArchPathRequest,
    profile: &FrontendAutoconfigProfileSpec,
    content: &Path,
    arguments: &[OsString],
    environment: &[(OsString, OsString)],
    cache_root: &Path,
) -> Result<(FrontendAutoconfigSession, Vec<OsString>)> {
    validate_unmodified_launch_context(arguments, environment)?;
    ensure_absolute(cache_root, "private cache root")?;
    let path_selection_inputs = path_selection_inputs(request)?;
    let mut request = request.clone();
    if let NativeRetroArchPathRequest::MacOs {
        application_bundle,
        install,
        ..
    } = &mut request
    {
        *install =
            macos_install_disposition_from_captures(application_bundle, &path_selection_inputs)?;
    }
    let captured_candidate_exists = |path: &Path| {
        path_selection_inputs
            .iter()
            .find(|capture| capture.requested_path == path)
            .is_some_and(|capture| capture.loadable)
    };
    let paths = resolve_native_retroarch_paths(&request, captured_candidate_exists)?;
    let main_config = path_selection_inputs
        .iter()
        .find(|capture| capture.requested_path == paths.main_config)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| CapturedConfig::capture_optional(&paths.main_config))?;
    let library = profile
        .retroarch_library
        .as_deref()
        .context("frontend_autoconfig profile needs an exact RetroArch library name")?;
    let (precedence_paths, fallback_path) =
        effective_core_options_candidates(&paths, main_config.text()?, library, content)?;
    let mut core_options_selection_inputs = Vec::new();
    for path in precedence_paths
        .iter()
        .chain(std::iter::once(&fallback_path))
    {
        if !core_options_selection_inputs
            .iter()
            .any(|capture: &CapturedConfig| capture.requested_path == *path)
        {
            core_options_selection_inputs.push(CapturedConfig::capture_optional(path)?);
        }
    }
    let effective_core_options_path = precedence_paths
        .iter()
        .find(|path| {
            core_options_selection_inputs
                .iter()
                .find(|capture| capture.requested_path == **path)
                .is_some_and(|capture| capture.path_valid)
        })
        .cloned()
        .unwrap_or(fallback_path);
    let effective_core_options = core_options_selection_inputs
        .iter()
        .find(|capture| capture.requested_path == effective_core_options_path)
        .context("Missing effective core-options capture")?
        .clone();
    ensure!(
        effective_core_options.loadable || effective_core_options.fingerprint.is_none(),
        "{} exists but cannot be snapshotted as a readable regular file",
        effective_core_options_path.display()
    );

    std::fs::create_dir_all(cache_root)
        .with_context(|| format!("Creating {}", cache_root.display()))?;
    let directory = tempfile::Builder::new()
        .prefix("frontend-autoconfig-")
        .tempdir_in(cache_root)?;
    secure_directory(directory.path())?;
    let artifacts = build_frontend_autoconfig(
        paths.host,
        profile,
        content,
        effective_core_options.text()?,
        directory.path(),
    )?;
    secure_directory(&directory.path().join("config"))?;
    secure_write(&artifacts.core_options_path, &artifacts.core_options)?;
    secure_write(&artifacts.append_config_path, &artifacts.append_config)?;
    let private_core_options = CapturedConfig::capture_required(&artifacts.core_options_path)?;
    let private_append_config = CapturedConfig::capture_required(&artifacts.append_config_path)?;
    let private_directories = vec![
        CapturedConfig::capture_optional(directory.path())?,
        CapturedConfig::capture_optional(&directory.path().join("config"))?,
    ];
    ensure!(private_core_options.text()? == artifacts.core_options);
    ensure!(private_append_config.text()? == artifacts.append_config);
    ensure_no_physical_mapping_keys(&artifacts.append_config)?;
    let prepared_arguments = attach_frontend_autoconfig(
        paths.host,
        arguments,
        environment,
        &artifacts.append_config_path,
        &artifacts.port_modes,
    )?;
    let session = FrontendAutoconfigSession {
        directory,
        request,
        verify_macos_bundle_seal: false,
        path_selection_inputs,
        core_options_selection_inputs,
        pinned_frontend: None,
        pinned_core: None,
        pinned_runtime_files: Vec::new(),
        pinned_content: None,
        prepared_arguments: None,
        prepared_environment: None,
        main_config,
        effective_core_options,
        private_append_config,
        private_core_options,
        private_directories,
        paths,
        effective_core_options_path,
        artifacts,
    };
    session.verify()?;
    Ok((session, prepared_arguments))
}

/// Production entry point for an audited native frontend/core pair. The exact
/// bytes are retained by length and digest and rechecked immediately before
/// child spawn. On macOS, the audited 1.22.2 identity also requires the app's
/// strict deep code-signature seal. On Windows this pins the complete 59-file
/// executable closure from the official RetroArch 1.19.1 distribution.
/// Unlisted executable-loadable files are rejected beside the frontend and
/// under its platform-plugin directory; ordinary mutable runtime data (such as
/// configs, cores, assets, saves, and logs) is allowed.
pub fn prepare_pinned_frontend_autoconfig_session(
    request: &NativeRetroArchPathRequest,
    core: &Path,
    expected_frontend_sha256: &str,
    expected_core_sha256: &str,
    profile: &FrontendAutoconfigProfileSpec,
    content: &Path,
    arguments: &[OsString],
    environment: &[(OsString, OsString)],
    cache_root: &Path,
) -> Result<(FrontendAutoconfigSession, Vec<OsString>)> {
    let executable = match request {
        NativeRetroArchPathRequest::MacOs { executable, .. }
        | NativeRetroArchPathRequest::Windows { executable, .. } => executable,
    };
    let frontend = CapturedFileIdentity::capture(executable)?;
    let core_capture = CapturedFileIdentity::capture(core)?;
    ensure!(
        frontend.sha256 == expected_frontend_sha256,
        "Native RetroArch executable is outside the audited frontend_autoconfig identity"
    );
    ensure!(
        core_capture.sha256 == expected_core_sha256,
        "Native Nestopia core is outside the audited frontend_autoconfig identity"
    );
    #[cfg(target_os = "macos")]
    if expected_frontend_sha256 == MACOS_RETROARCH_1_22_2_SHA256 {
        let NativeRetroArchPathRequest::MacOs {
            application_bundle, ..
        } = request
        else {
            bail!("The audited macOS RetroArch identity requires a macOS bundle request");
        };
        verify_macos_bundle_seal(application_bundle)?;
    }
    let pinned_runtime_files = match request {
        NativeRetroArchPathRequest::Windows { .. } => {
            capture_windows_retroarch_distribution(executable, &frontend)?
        }
        NativeRetroArchPathRequest::MacOs { .. } => Vec::new(),
    };
    let (mut session, arguments) = prepare_frontend_autoconfig_session(
        request,
        profile,
        content,
        arguments,
        environment,
        cache_root,
    )?;
    session.pinned_frontend = Some(frontend);
    session.pinned_core = Some(core_capture);
    session.pinned_runtime_files = pinned_runtime_files;
    session.pinned_content = Some(CapturedFileIdentity::capture(content)?);
    session.prepared_arguments = Some(arguments.clone());
    session.prepared_environment = Some(environment.to_vec());
    session.verify_macos_bundle_seal = matches!(request, NativeRetroArchPathRequest::MacOs { .. })
        && expected_frontend_sha256 == MACOS_RETROARCH_1_22_2_SHA256;
    session.verify()?;
    Ok((session, arguments))
}

#[cfg(target_os = "macos")]
fn verify_macos_bundle_seal(application_bundle: &Path) -> Result<()> {
    use std::process::{Command, Stdio};

    ensure_absolute(application_bundle, "RetroArch application bundle")?;
    let status = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(application_bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Executing the macOS code-signature verifier")?;
    ensure!(
        status.success(),
        "Native macOS RetroArch bundle failed strict deep code-signature verification"
    );
    Ok(())
}

fn capture_windows_retroarch_distribution(
    executable: &Path,
    frontend: &CapturedFileIdentity,
) -> Result<Vec<CapturedFileIdentity>> {
    ensure!(
        WINDOWS_RETROARCH_1_19_1_DISTRIBUTION.len() == 59,
        "Audited RetroArch Windows distribution closure is incomplete"
    );
    let root = executable
        .parent()
        .context("Native Windows RetroArch executable has no parent directory")?;
    let resolved_root = std::fs::canonicalize(root)
        .with_context(|| format!("Resolving RetroArch runtime root {}", root.display()))?;
    verify_windows_runtime_loadable_members(executable)?;
    let mut relative_paths = BTreeSet::new();
    let mut captures = Vec::with_capacity(WINDOWS_RETROARCH_1_19_1_DISTRIBUTION.len());
    for expected in WINDOWS_RETROARCH_1_19_1_DISTRIBUTION {
        let relative = Path::new(expected.relative_path);
        ensure!(
            !expected.relative_path.is_empty()
                && relative
                    .components()
                    .all(|component| matches!(component, std::path::Component::Normal(_))),
            "Audited RetroArch runtime path is not a strict relative path: {}",
            expected.relative_path
        );
        ensure!(
            relative_paths.insert(expected.relative_path.to_ascii_lowercase()),
            "Audited RetroArch runtime path is duplicated: {}",
            expected.relative_path
        );
        let path = root.join(relative);
        ensure!(
            std::fs::symlink_metadata(&path)?.file_type().is_file(),
            "Audited RetroArch runtime member is not a direct regular file: {}",
            path.display()
        );
        let capture = CapturedFileIdentity::capture_expected(&path, expected)?;
        ensure!(
            capture.resolved_path.starts_with(&resolved_root),
            "Audited RetroArch runtime member resolves outside its root: {}",
            path.display()
        );
        captures.push(capture);
    }
    let distribution_frontend = WINDOWS_RETROARCH_1_19_1_DISTRIBUTION
        .iter()
        .position(|file| file.relative_path == "retroarch.exe")
        .context("Audited RetroArch Windows closure omits retroarch.exe")?;
    ensure!(
        captures[distribution_frontend].resolved_path == frontend.resolved_path,
        "Selected RetroArch executable is not the audited distribution frontend"
    );
    verify_windows_runtime_loadable_members(executable)?;
    Ok(captures)
}

fn verify_windows_runtime_loadable_members(executable: &Path) -> Result<()> {
    let root = executable
        .parent()
        .context("Native Windows RetroArch executable has no parent directory")?;
    let allowed = WINDOWS_RETROARCH_1_19_1_DISTRIBUTION
        .iter()
        .map(|file| file.relative_path.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    verify_no_unexpected_loadable_files(root, root, &allowed, false)?;
    let platforms = root.join("platforms");
    ensure!(
        platforms.is_dir(),
        "Audited RetroArch Windows platform-plugin directory is missing"
    );
    verify_no_unexpected_loadable_files(root, &platforms, &allowed, true)
}

fn verify_no_unexpected_loadable_files(
    root: &Path,
    directory: &Path,
    allowed: &BTreeSet<String>,
    recurse: bool,
) -> Result<()> {
    for entry in std::fs::read_dir(directory).with_context(|| {
        format!(
            "Reading RetroArch runtime directory {}",
            directory.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if recurse && file_type.is_dir() {
            verify_no_unexpected_loadable_files(root, &path, allowed, true)?;
            continue;
        }
        if directory == root && file_type.is_dir() {
            continue;
        }
        let loadable_extension =
            path.extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("dll") || extension.eq_ignore_ascii_case("exe")
                });
        if !loadable_extension {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .context("RetroArch runtime member escaped its root")?;
        let relative = relative
            .to_str()
            .context("RetroArch runtime member path is not Unicode")?
            .replace('\\', "/")
            .to_ascii_lowercase();
        ensure!(
            allowed.contains(&relative),
            "Unexpected executable-loadable file in the audited RetroArch runtime: {}",
            path.display()
        );
    }
    Ok(())
}

fn path_selection_inputs(request: &NativeRetroArchPathRequest) -> Result<Vec<CapturedConfig>> {
    match request {
        NativeRetroArchPathRequest::Windows {
            executable,
            roaming_app_data,
            ..
        } => Ok(vec![
            CapturedConfig::capture_optional(
                &executable
                    .parent()
                    .context("Native Windows RetroArch executable has no parent directory")?
                    .join("retroarch.cfg"),
            )?,
            CapturedConfig::capture_optional(&roaming_app_data.join("retroarch.cfg"))?,
        ]),
        NativeRetroArchPathRequest::MacOs {
            application_bundle, ..
        } => {
            let root = application_bundle
                .parent()
                .context("Native macOS RetroArch application bundle has no containing directory")?;
            Ok(vec![
                CapturedConfig::capture_optional(&root.join("portable.txt"))?,
                CapturedConfig::capture_optional(&application_bundle.join("Contents/Info.plist"))?,
            ])
        }
    }
}

fn secure_directory(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(windows)]
    secure_windows_acl(path)?;
    Ok(())
}

fn secure_write(path: &Path, contents: &str) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("Creating private {}", path.display()))?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(windows)]
    secure_windows_acl(path)?;
    Ok(())
}

fn verify_private_security(
    directories: &[CapturedConfig],
    files: &[&CapturedConfig],
) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for directory in directories {
            let metadata = std::fs::symlink_metadata(&directory.requested_path)?;
            ensure!(
                metadata.is_dir() && metadata.permissions().mode() & 0o777 == 0o700,
                "Private frontend_autoconfig directory permissions changed: {}",
                directory.requested_path.display()
            );
        }
        for file in files {
            let metadata = std::fs::symlink_metadata(&file.requested_path)?;
            ensure!(
                metadata.is_file() && metadata.permissions().mode() & 0o777 == 0o600,
                "Private frontend_autoconfig file permissions changed: {}",
                file.requested_path.display()
            );
        }
    }
    #[cfg(windows)]
    {
        for path in directories
            .iter()
            .map(|capture| &capture.requested_path)
            .chain(files.iter().map(|capture| &capture.requested_path))
        {
            // Reapply the protected owner+SYSTEM DACL at the final pre-spawn
            // verification boundary so inherited or added ACEs cannot persist.
            secure_windows_acl(path)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn secure_windows_acl(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
        SetFileSecurityW,
    };

    // Protected DACL: the owning user and LocalSystem only. OI/CI propagate the
    // same policy to any subsequently created session children.
    let sddl: Vec<u16> = "D:P(A;OICI;FA;;;OW)(A;OICI;FA;;;SY)\0"
        .encode_utf16()
        .collect();
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    };
    ensure!(
        converted != 0,
        "Creating private Windows security descriptor failed: {}",
        std::io::Error::last_os_error()
    );
    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let applied = unsafe {
        SetFileSecurityW(
            wide_path.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
    };
    let apply_error = std::io::Error::last_os_error();
    unsafe {
        LocalFree(descriptor);
    }
    ensure!(
        applied != 0,
        "Protecting private Windows path {} failed: {apply_error}",
        path.display()
    );
    Ok(())
}

pub fn build_frontend_autoconfig(
    host: FrontendAutoconfigHost,
    profile: &FrontendAutoconfigProfileSpec,
    content: &Path,
    baseline_core_options: &str,
    private_root: &Path,
) -> Result<FrontendAutoconfigArtifacts> {
    ensure_absolute(content, "content path")?;
    ensure_absolute(private_root, "private session root")?;
    validate_allowlisted_profile(profile, content)?;
    let core_options = core_options_overlay(baseline_core_options, &profile.core_options)?;
    let core_options_path = private_root.join("core-options.cfg");
    let private_config_directory = private_root.join("config");
    let append_config_path = private_root.join("frontend-autoconfig.cfg");
    let mut append_config = String::from(
        "# Lunchbox frontend_autoconfig profile. RetroArch retains host input discovery and bindings.\n\
auto_remaps_enable = \"false\"\n\
auto_overrides_enable = \"false\"\n\
config_save_on_exit = \"false\"\n\
remap_save_on_exit = \"false\"\n\
game_specific_options = \"false\"\n\
global_core_options = \"false\"\n",
    );
    append_config.push_str(&format!(
        "core_options_path = \"{}\"\nrgui_config_directory = \"{}\"\n",
        encode_config_path(host, &core_options_path)?,
        encode_config_path(host, &private_config_directory)?,
    ));

    let mut port_modes = Vec::with_capacity(profile.frontend_ports);
    for port in 1..=profile.frontend_ports {
        let device = if port <= profile.max_players {
            profile
                .port_devices
                .get(&port)
                .copied()
                .unwrap_or(profile.default_device)
        } else {
            profile.port_devices.get(&port).copied().unwrap_or(0)
        };
        port_modes.push(FrontendPortMode {
            frontend_port: port,
            libretro_device: device,
        });
        append_config.push_str(&format!("input_libretro_device_p{port} = \"{device}\"\n"));
    }
    let highest_active = profile
        .port_devices
        .iter()
        .filter_map(|(port, device)| (*device != 0).then_some(*port))
        .chain(std::iter::once(profile.max_players))
        .max()
        .context("frontend_autoconfig profile has no active ports")?;
    append_config.push_str(&format!("input_max_users = \"{highest_active}\"\n"));
    ensure_no_physical_mapping_keys(&append_config)?;
    Ok(FrontendAutoconfigArtifacts {
        append_config_path,
        append_config,
        core_options_path,
        core_options,
        port_modes,
        description: format!(
            "RetroArch frontend_autoconfig applied {}; RetroArch retained host controller discovery and bindings. Physical controller identity, order, and mapping were not verified.",
            profile.id
        ),
    })
}

fn validate_allowlisted_profile(
    profile: &FrontendAutoconfigProfileSpec,
    content: &Path,
) -> Result<()> {
    ensure!(
        !profile.dynamic_profile,
        "Dynamic RetroArch profiles require their dedicated launch adapter"
    );
    ensure!(
        !profile.special_preparation,
        "Special RetroArch profile preparation is excluded from frontend_autoconfig"
    );
    ensure!(
        profile.content_guard.is_none(),
        "Content-guarded profile is not allowlisted for frontend_autoconfig"
    );
    ensure!(
        !profile.requires_fresh_start,
        "Fresh-start profile needs effective state resolution before frontend_autoconfig"
    );
    ensure!(
        !profile.has_player_topology,
        "Option-dependent player topology is not allowlisted for frontend_autoconfig"
    );
    let expected_players = match profile.id.as_str() {
        NESTOPIA_NES_TWO_PLAYER_PROFILE => 2,
        NESTOPIA_NES_FOUR_PLAYER_PROFILE => 4,
        _ => bail!("RetroArch profile is not in the conservative frontend_autoconfig allowlist"),
    };
    ensure!(
        profile.core == "nestopia"
            && profile.target_layout == "nes"
            && profile.transport == "retropad"
            && profile.retroarch_library.as_deref() == Some("Nestopia")
            && profile.explicit_selection
            && profile.platforms
                == BTreeSet::from([
                    "NES".to_owned(),
                    "Nintendo NES".to_owned(),
                    "Nintendo Entertainment System".to_owned(),
                    "Nintendo - Nintendo Entertainment System".to_owned(),
                    "Nintendo Famicom".to_owned(),
                ])
            && profile.content_extensions
                == BTreeSet::from(["nes".to_owned(), "unf".to_owned(), "unif".to_owned()])
            && profile.frontend_ports == 4
            && profile.max_players == expected_players
            && profile.default_device == 257
            && profile.port_devices.is_empty()
            && profile.core_options
                == BTreeMap::from([("nestopia_button_shift".to_owned(), "disabled".to_owned())]),
        "Nestopia frontend_autoconfig profile differs from its reviewed ordinary-pad contract"
    );
    let extension = content
        .extension()
        .and_then(OsStr::to_str)
        .context("Nestopia frontend_autoconfig content needs a UTF-8 extension")?;
    ensure!(
        matches!(
            extension.to_ascii_lowercase().as_str(),
            "nes" | "unf" | "unif"
        ),
        "Nestopia frontend_autoconfig covers ordinary NES cartridge content only"
    );
    Ok(())
}

fn core_options_overlay(baseline: &str, options: &BTreeMap<String, String>) -> Result<String> {
    reject_includes(baseline, "core-options file")?;
    let mut output = baseline
        .lines()
        .filter(|line| {
            !line
                .split_once('=')
                .is_some_and(|(key, _)| options.contains_key(key.trim()))
        })
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    for (key, value) in options {
        ensure!(
            !key.is_empty()
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
                && !value.contains(['"', '\n', '\r', '\\']),
            "Invalid core option in frontend_autoconfig profile"
        );
        output.push_str(&format!("{key} = \"{value}\"\n"));
    }
    Ok(output)
}

pub fn ensure_no_physical_mapping_keys(config: &str) -> Result<()> {
    for line in config.lines() {
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        ensure!(
            !matches!(key, "input_joypad_driver" | "input_autodetect_enable")
                && !key.ends_with("_joypad_index")
                && !(key.starts_with("input_player")
                    && (key.ends_with("_btn") || key.ends_with("_axis"))),
            "frontend_autoconfig must not emit physical controller key {key}"
        );
    }
    Ok(())
}

fn encode_config_path(host: FrontendAutoconfigHost, path: &Path) -> Result<String> {
    let text = path
        .to_str()
        .context("frontend_autoconfig path must be Unicode")?;
    ensure!(
        !text.chars().any(char::is_control) && !text.contains('"'),
        "frontend_autoconfig path cannot be quoted safely"
    );
    Ok(if host == FrontendAutoconfigHost::Windows {
        text.replace('\\', "/")
    } else {
        ensure!(
            !text.contains('\\'),
            "macOS frontend_autoconfig path contains an unsupported escape"
        );
        text.to_owned()
    })
}

fn reject_includes(text: &str, source: &str) -> Result<()> {
    ensure!(
        !text
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included {source} needs effective resolution before frontend_autoconfig"
    );
    Ok(())
}

fn config_value(text: &str, key: &str) -> Result<Option<String>> {
    let mut result = None;
    for line in text.lines() {
        let Some((name, value)) = line.trim().split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        ensure!(
            name.ends_with(char::is_whitespace),
            "RetroArch requires whitespace before '=' in {key}"
        );
        ensure!(result.is_none(), "Duplicate {key} setting");
        let value = value.trim();
        let (value, rest) = if let Some(quoted) = value.strip_prefix('"') {
            quoted
                .split_once('"')
                .with_context(|| format!("Unterminated {key} setting"))?
        } else {
            let end = value
                .find(|character: char| character.is_ascii_whitespace() || character == '#')
                .unwrap_or(value.len());
            ensure!(end > 0, "Missing {key} value");
            (&value[..end], &value[end..])
        };
        ensure!(
            rest.trim().is_empty() || rest.trim_start().starts_with('#'),
            "Unresolved {key} setting suffix"
        );
        result = Some(value.to_owned());
    }
    Ok(result)
}

fn config_bool(text: &str, key: &str, default: bool) -> Result<bool> {
    match config_value(text, key)? {
        None => Ok(default),
        Some(value) if value == "true" => Ok(true),
        Some(value) if value == "false" => Ok(false),
        Some(_) => bail!("Unresolved {key} boolean setting"),
    }
}
