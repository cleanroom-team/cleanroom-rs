// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;

use crate::context::SystemContext;

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

pub fn create_script(
    phase: &crate::Phases,
    ctx: &SystemContext,
) -> anyhow::Result<Option<PathBuf>> {
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

    script += include_str!("header.sh");
    assert!(script.ends_with('\n'));

    script += "### <system_environment>\n";
    for ce in ctx.iter().filter(|ce| !ce.is_internal) {
        let value = escape(&ce.value);
        if ce.is_read_only {
            script += &format!("{}=\"{}\"; readonly {}\n", ce.name, value, ce.name);
        } else {
            script += &format!("{}=\"{}\"\n", ce.name, value);
        }
    }
    script += "### </system_environment>\n\n";

    script += include_str!("footer.sh");
    assert!(script.ends_with('\n'));

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

        eprintln!("input   : >{input}<");
        eprintln!("expected: >{expected}<");
        eprintln!("got     : >{result}<");

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
