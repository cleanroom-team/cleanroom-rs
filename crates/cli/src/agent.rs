// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

// cSpell: ignore logname

use anyhow::Context;

pub fn run(command_prefix: &str, phase: &crate::Phases) -> anyhow::Result<()> {
    for sub_phase in crate::SubPhases::iter() {
        let mut child = std::process::Command::new("/clrm/busybox")
            .arg("sh")
            .arg("-e")
            .arg("/clrm/script.sh")
            .arg(command_prefix)
            .arg(phase.to_string())
            .arg(sub_phase.to_string())
            .spawn()
            .context("Failed to start agent script")?;

        let exit_status = child
            .wait()
            .context("Failed running the agent script for {phase}/{sub_phase}")?;
        if !exit_status.success() {
            return Err(anyhow::anyhow!(format!(
                "Agent script in phase {phase}/{sub_phase} quit with error"
            )));
        }
    }
    Ok(())
}
