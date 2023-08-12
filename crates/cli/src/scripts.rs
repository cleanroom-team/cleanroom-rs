// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;

use crate::{context::SystemContext, printer::Printer};

pub fn create_script(
    phase: &crate::Phases,
    printer: &Printer,
    ctx: &SystemContext,
) -> anyhow::Result<Option<PathBuf>> {
    printer.h2("Create phase script", true);
    let phase_script = ctx
        .agent_script_directory()
        .unwrap()
        .join(format!("{phase:?}.sh").to_lowercase());

    printer.debug(&format!(
        "Phase script path for {phase:?}: {phase_script:?}"
    ));

    let header = include_str!("header.sh");
    let mut output = std::fs::File::create(&phase_script).context(format!(
        "Failed to write phase script file {phase_script:?}"
    ))?;
    write!(output, "{header}").context("")?;

    Ok(Some(phase_script))
}
