//! Explicit refusal for OdySim controller serialization.
//!
//! OdySim is distributed as per-game LÖVE/Windows archives.  The preserved
//! archive contains an executable and DLLs (and game-specific readmes), but no
//! source-pinned mapping grammar or persistent controller profile.

pub(crate) const SOURCE_URL: &str = "https://archive.org/details/OdySim";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "OdySim controller mapping is unavailable: the archived LÖVE artifact exposes runtime keyboard controls but no source-backed persistent host-controller profile"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn refuses_unverified_profile() {
        assert!(super::refusal().is_err());
        assert!(super::SOURCE_URL.starts_with("https://"));
    }
}
