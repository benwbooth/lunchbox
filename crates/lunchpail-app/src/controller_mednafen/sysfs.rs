//! Native sysfs input identity; does not open or write controller devices.
use anyhow::{Context, Result, ensure};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

fn input_base() -> Result<PathBuf> {
    for path in [
        "/sys/subsystem/input",
        "/sys/bus/input",
        "/sys/block/input",
        "/sys/class/input",
    ] {
        match std::fs::metadata(path) {
            Ok(_) => return Ok(path.into()),
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    || error.raw_os_error() == Some(libc::ENOTDIR) => {}
            Err(error) => return Err(error.into()),
        }
    }
    anyhow::bail!("Mednafen cannot locate native sysfs input subsystem")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Identity {
    pub(crate) input: PathBuf,
    pub(crate) components: [u16; 4],
    joystick: PathBuf,
}

impl Identity {
    pub(crate) fn event_path(&self) -> Result<PathBuf> {
        let names = super::enumeration::names(&self.input, "event")?
            .context("Mednafen sysfs input directory disappeared")?;
        let name = names
            .first()
            .context("Mednafen joystick has no corresponding event node")?;
        let link = Path::new("/sys/class/input").join(name).canonicalize()?;
        ensure!(
            link.parent() == Some(self.input.as_path()),
            "Mednafen event node belongs to a different input device"
        );
        Ok(Path::new("/dev/input").join(name))
    }

    pub(crate) fn capture(joystick: &Path) -> Result<Self> {
        let name = joystick
            .file_name()
            .context("Mednafen joystick name is absent")?;
        let resolved = input_base()?.join(name).canonicalize()?;
        let input = resolved
            .ancestors()
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.strip_prefix("input"))
                    .is_some_and(|suffix| {
                        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                    })
            })
            .context("Mednafen joystick has no native input parent")?
            .to_owned();
        let mut components = [0; 4];
        for (index, name) in ["bustype", "vendor", "product", "version"]
            .iter()
            .enumerate()
        {
            let mut text = String::new();
            std::fs::File::open(input.join("id").join(name))?
                .take(129)
                .read_to_string(&mut text)?;
            ensure!(
                text.len() <= 128,
                "Mednafen sysfs identity exceeds size limit"
            );
            let value = u32::from_str_radix(text.trim().trim_start_matches("0x"), 16)?;
            components[index] = (value & 0xffff) as u16;
        }
        Ok(Self {
            input,
            components,
            joystick: joystick.to_owned(),
        })
    }

    pub(crate) fn base_id(&self, map: &super::physical::Map) -> Result<[u8; 16]> {
        let [bus, vendor, product, version] = self.components;
        Ok(super::identity::linux_base(
            bus,
            vendor,
            product,
            version,
            map.axes.len().try_into()?,
            map.buttons.len().try_into()?,
        ))
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            Self::capture(&self.joystick)? == *self,
            "Mednafen sysfs controller identity changed"
        );
        Ok(())
    }
}
