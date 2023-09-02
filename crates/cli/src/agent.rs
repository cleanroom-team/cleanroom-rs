// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

// cSpell: ignore logname

use anyhow::Context;

pub fn run(command_prefix: &str, phase: &crate::Phases) -> anyhow::Result<()> {
    let agent_script = "/tmp/clrm/script.sh";
    let mut child = std::process::Command::new("/tmp/clrm/busybox")
        .arg("sh")
        .arg("-e")
        .arg(agent_script)
        .arg(command_prefix)
        .arg(phase.to_string())
        .spawn()
        .context(format!(
            "The agent failed to start the agent script in {agent_script}"
        ))?;

    let exit_status = child
        .wait()
        .context("Failed running the agent script for {phase}")?;
    if !exit_status.success() {
        let exit_code = exit_status
            .code()
            .map(|c| format!("{c}"))
            .unwrap_or_else(|| String::from("<unknown>"));
        Err(anyhow::anyhow!(format!(
            "Agent script in phase {phase} quit with exit_code {exit_code}"
        )))
    } else {
        Ok(())
    }
}
