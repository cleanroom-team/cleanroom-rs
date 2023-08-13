// Copyright © Tobias Hunger <tobiias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;

use crate::context::SystemContext;
use crate::{Phases, SubPhases};

struct Section {
    name: String,
    contents: String,
}

impl Section {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            contents: String::new(),
        }
    }

    fn push_str(&mut self, rhs: &str) {
        let to_add = if rhs.starts_with("#!/usr/bin/sh") || rhs.starts_with("#!/bin/sh") {
            if let Some((_, rest)) = rhs.split_once('\n') {
                rest
            } else {
                ""
            }
        } else {
            rhs
        };
        self.contents += to_add;
    }

    fn extract(self) -> String {
        let mut result = String::new();
        result += &format!("### <{}>\n", self.name);
        result += &self.contents;
        if !result.ends_with('\n') {
            result.push('\n');
        }
        result += &format!("### </{}>\n\n", self.name);

        result
    }
}

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

fn script_add_header() -> Section {
    let mut section = Section::new("header");
    section.push_str(include_str!("header.sh"));
    section
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
) -> anyhow::Result<Section> {
    let mut section = Section::new("command definition");

    let cmd = ctx.command_manager().command(name)?;

    for sub_phase in SubPhases::iter() {
        if let Some(script) = cmd.snippet(phase, &sub_phase) {
            section.push_str(&format!(
                "{}() {{\n",
                command_function_name(name, &sub_phase)
            ));
            for i in cmd.inputs() {
                section.push_str(&format!("    {}=\"${{1}}\"; shift\n", i.name()));
            }
            section.push_str(&format!("\n{script}\n}}\n\n"));
        }
    }

    Ok(section)
}

fn script_add_system_environment(ctx: &SystemContext) -> Section {
    let mut section = Section::new("system environment");
    for ce in ctx.iter().filter(|ce| !ce.is_internal) {
        let value = escape(&ce.value);
        if ce.is_read_only {
            section.push_str(&format!(
                "{}=\"{}\"; readonly {}\n",
                ce.name, value, ce.name
            ));
        } else {
            section.push_str(&format!("{}=\"{}\"\n", ce.name, value));
        }
    }
    section
}

fn script_add_footer() -> Section {
    let mut section = Section::new("footer");
    section.push_str(include_str!("footer.sh"));
    section
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

    script += &script_add_header().extract();
    script += &script_add_command_definitions("test", phase, ctx)?.extract();
    script += &script_add_system_environment(ctx).extract();
    script += &script_add_footer().extract();

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
