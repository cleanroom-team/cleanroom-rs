// Copyright © Tobias Hunger <tobiias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;

use crate::context::RunContext;

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
        self.contents.push_str(to_add);
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

fn script_add_phase_definitions() -> Section {
    let mut section = Section::new("phase definitions");
    for p in crate::Phases::iter().map(|p| p.to_string()) {
        let pu = p.to_uppercase();
        section.push_str(&format!("PHASE_{pu}=\"{p}\"\nreadonly PHASE_{pu}\n"));
    }
    section
}

fn script_add_command_definitions(ctx: &RunContext) -> anyhow::Result<Section> {
    let mut section = Section::new("command definition");

    for (name, cmd) in ctx.command_manager().commands() {
        section.push_str(&format!("{name}() {{\n"));
        section.push_str(&format!("    status \"{name}\"\n"));
        for i in cmd.inputs() {
            section.push_str(&format!("    {}=\"${{1}}\"; shift\n", i.name()));
        }
        section.push_str(&format!("\n{}\n}}\n\n", cmd.script));
    }

    Ok(section)
}

fn script_add_system_environment(ctx: &RunContext) -> Section {
    let mut section = Section::new("system environment");
    for ce in ctx.iter().filter(|ce| !ce.is_internal) {
        let value = escape(&ce.value);
        if ce.is_read_only {
            section.push_str(&format!(
                "{}=\"{}\"\nreadonly {}\n",
                ce.name, value, ce.name
            ));
        } else {
            section.push_str(&format!("{}=\"{}\"\n", ce.name, value));
        }
    }
    section
}

fn script_add_pre_command() -> Section {
    let mut section = Section::new("pre_command");
    section.push_str(include_str!("pre_command.sh"));
    section
}

fn script_add_command(start_command: &str) -> Section {
    let mut section = Section::new("command");
    section.push_str(start_command);
    section
}

fn script_add_footer() -> Section {
    let mut section = Section::new("footer");
    section.push_str(include_str!("footer.sh"));
    section
}

pub fn create_script(ctx: &RunContext, start_command: &str) -> anyhow::Result<PathBuf> {
    let p = ctx.printer();
    let script_path = ctx.scratch_directory().unwrap().join("script.sh");

    let mut script_contents = String::from("#!/bin/sh -e\n");

    script_contents += &script_add_header().extract();
    script_contents += &script_add_phase_definitions().extract();
    script_contents += &script_add_command_definitions(ctx)?.extract();
    script_contents += &script_add_system_environment(ctx).extract();
    script_contents += &script_add_pre_command().extract();
    script_contents += &script_add_command(start_command).extract();
    script_contents += &script_add_footer().extract();

    let mut output = std::fs::File::create(&script_path)
        .context(format!("Failed to write agent script file {script_path:?}"))?;
    write!(output, "{script_contents}")
        .context(format!("Failed to write agent script into {script_path:?}"))?;

    p.trace(&format!("Full agent script at {script_path:?}"));

    Ok(script_path)
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
