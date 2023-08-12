// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

// cSpell: ignore logname

use anyhow::Context;

pub fn run(command_prefix: &str) -> anyhow::Result<()> {
    let mut child = std::process::Command::new("/clrm/busybox")
        .arg("sh")
        .arg("-e")
        .arg("/clrm/script.sh")
        .arg(command_prefix)
        .spawn()
        .context("Failed to start phase script")?;

    let exit_status = child.wait().context("Failed running the phase script")?;
    if exit_status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Phase script quit with error"))
    }
}
