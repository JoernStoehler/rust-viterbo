//! Minimal process runner. Prefer scripts/safe.sh for robust group-kill.
//! This is a placeholder that can be extended to use setsid/prctl via `nix`.

use std::process::Command;
use anyhow::{bail, Result};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().ok_or_else(|| anyhow::anyhow!("usage: safe-exec <program> [args...]"))?;
    let status = Command::new(cmd).args(args).status()?;
    if !status.success() {
        bail!("child exited with status {:?}", status.code());
    }
    Ok(())
}

