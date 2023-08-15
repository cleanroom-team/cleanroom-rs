// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

// cSpell: ignore logname

use anyhow::Context;

pub fn run(command_prefix: &str, phase: &crate::Phases) -> anyhow::Result<()> {
    let mut child = std::process::Command::new("/clrm/busybox")
        .arg("sh")
        .arg("-e")
        .arg("/clrm/script.sh")
        .arg(command_prefix)
        .arg(phase.to_string())
        .spawn()
        .context("Failed to start agent script")?;

    let exit_status = child
        .wait()
        .context("Failed running the agent script for {phase}")?;
    if !exit_status.success() {
        Err(anyhow::anyhow!(format!(
            "Agent script in phase {phase} quit with error"
        )))
    } else {
        Ok(())
    }
}
