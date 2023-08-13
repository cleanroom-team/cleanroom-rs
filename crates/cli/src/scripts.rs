// Copyright © Tobias Hunger <tobiias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;

use crate::context::SystemContext;
use crate::{Phases, SubPhases};

fn escape(input: &str) -> String {
    let mut result = String::new();
    for c in input.chars() {
        match c {
            '\\' => {
                result.push(c);
                result.push(c);
            }
            '"' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
}

fn script_add_header() -> String {
    let mut result = String::from("### <header>\n\n");
    result += include_str!("header.sh");
    result += "### </header>\n\n";

    result = result.replace("#!/usr/bin/sh -e\n\n", "");
    result
}

fn command_function_name(name: &str, sub_phase: &SubPhases) -> String {
    let name_extra = {
        if sub_phase == &SubPhases::Main {
            String::new()
        } else {
            format!("_{}", sub_phase)
        }
    };
    format!("cmd_{name}{name_extra}")
}

fn script_add_command_definitions(
    name: &str,
    phase: &Phases,
    ctx: &SystemContext,
) -> anyhow::Result<String> {
    let mut result = String::from("### <command definitions>\n\n");

    let cmd = ctx.command_manager().command(name)?;

    for sub_phase in SubPhases::iter() {
        if let Some(script) = cmd.snippet(phase, &sub_phase) {
            result += &format!("{}() {{\n", command_function_name(name, &sub_phase));
            for i in cmd.inputs() {
                result += &format!("    {}=\"${{1}}\"; shift\n", i.name());
            }
            result += &format!("\n{script}\n}}\n\n");
        }
    }

    result += "### </command definitions>\n\n";
    Ok(result)
}

fn script_add_system_environment(ctx: &SystemContext) -> String {
    let mut result = String::from("### <system environment>\n\n");
    for ce in ctx.iter().filter(|ce| !ce.is_internal) {
        let value = escape(&ce.value);
        if ce.is_read_only {
            result += &format!("{}=\"{}\"; readonly {}\n", ce.name, value, ce.name);
        } else {
            result += &format!("{}=\"{}\"\n", ce.name, value);
        }
    }
    result += "### </system environment>\n\n";
    result
}

fn script_add_footer() -> String {
    let mut result = String::from("### <footer>\n\n");
    result += include_str!("footer.sh");
    result += "### </footer>\n\n";

    result = result.replace("#!/usr/bin/sh -e\n\n", "");
    result
}

pub fn create_script(phase: &Phases, ctx: &SystemContext) -> anyhow::Result<Option<PathBuf>> {
    let p = ctx.printer();
    p.h2("Create phase script", true);
    let phase_script = ctx
        .agent_script_directory()
        .unwrap()
        .join(format!("{phase}.sh"));

    p.debug(&format!(
        "Phase script path for {phase:?}: {phase_script:?}"
    ));

    let mut script = String::new();

    script += &script_add_header();
    script += &script_add_command_definitions("test", phase, ctx)?;
    script += &script_add_system_environment(ctx);
    script += &script_add_footer();

    let mut output = std::fs::File::create(&phase_script).context(format!(
        "Failed to write phase script file {phase_script:?}"
    ))?;
    write!(output, "{script}").context(format!("Failed to write script into {phase_script:?}"))?;

    p.trace(&format!("Full phase script:\n{script}"));

    Ok(Some(phase_script))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell_escape(input: &str, expected: &str) {
        let result = escape(input);

        assert_eq!(&result, expected);
    }

    #[test]
    fn test_shell_escape_unchanged() {
        shell_escape(
            r#"foobar 1, 2, 3, 4, XYZ # bar foo"#,
            r#"foobar 1, 2, 3, 4, XYZ # bar foo"#,
        );
    }

    #[test]
    fn test_shell_escape_quoted_double_quotes() {
        shell_escape(r#"foo "b\"a\"z" bar"#, r#"foo \"b\\\"a\\\"z\" bar"#);
    }
}
