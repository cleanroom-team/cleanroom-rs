// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::printer::Printer;

use anyhow::Context;
use contained_command::{Binding, Command, Nspawn};

use std::path::{Path, PathBuf};

// - Constants:
// ----------------------------------------------------------------------

const DEFAULT_MACHINE_ID: [u8; 32] = [
    b'0', b'b', b'f', b'9', b'5', b'b', b'b', b'7', b'7', b'1', b'3', b'6', b'4', b'e', b'f', b'9',
    b'9', b'7', b'e', b'1', b'd', b'f', b'5', b'e', b'b', b'3', b'b', b'2', b'6', b'4', b'2', b'2',
];

fn parse_stdout(
    m: &str,
    command_prefix: &str,
    printer: &Printer,
    ctx: &mut crate::context::Context,
) -> bool {
    let Some(cmd) = m.strip_prefix(command_prefix) else {
        return false;
    };

    printer.trace(&format!("Processing {}", cmd));
    if let Some(to_set) = cmd.strip_prefix("SET ") {
        if let Some((k, v)) = to_set.split_once('=') {
            let k = k.trim_matches('"');
            let v = v.trim_matches('"');
            if ctx.set(k, v).is_err() {
                printer.error(&format!(
                    "Could not parse arguments after SET: {k:?} is not a valid variable name"
                ));
            }
        } else {
            printer.error(&format!(
                "Could not parse arguments after SET: No '=' found in {to_set:?}"
            ));
        }
    } else {
        printer.error(&format!("Agent asked to process unknown command {cmd:?}"))
    }
    true
}

#[allow(clippy::needless_pass_by_ref_mut)] // FIXME: It's not useless: It's passed on to parse_stdout!
pub async fn run_agent(
    printer: &Printer,
    root_directory: &Path,
    agent_executable: &Path,
    ctx: &mut crate::context::Context,
) -> anyhow::Result<()> {
    printer.h1("Run Agent", true);

    let runner = Nspawn::default_runner(root_directory)?
        .machine_id(DEFAULT_MACHINE_ID)
        .share_users()
        .binding(Binding::ro(&agent_executable, &PathBuf::from("/agent")));

    let command_prefix = uuid::Uuid::new_v4().to_string();
    let command = {
        let mut command = Command::new("/agent");
        command.arg("agent");
        command.arg(&format!("--command-prefix={command_prefix}"));

        command.envs(ctx.iter());
        command
    };

    let command_prefix = format!("{command_prefix}: ");
    runner
        .run(
            &command,
            &|m| printer.trace(m),
            &|m| printer.error(m),
            &mut |m| {
                if !parse_stdout(m, &command_prefix, printer, ctx) {
                    printer.print_stdout(m);
                }
            },
            &mut |m| printer.print_stderr(m),
        )
        .await
        .context("Failed to containerize")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::printer::LogLevel;

    use super::*;

    fn test_parse_stdout(
        input: &str,
        command_prefix: &str,
        expect_handled: bool,
        expect_error: bool,
    ) -> crate::context::Context {
        let printer = Printer::new(&LogLevel::Off, false);

        let mut ctx = crate::context::Context::default();
        ctx.set("FOO", "bar").unwrap();
        ctx.set("BAR", "foo").unwrap();

        assert_eq!(
            parse_stdout(input, command_prefix, &printer, &mut ctx),
            expect_handled
        );

        assert_eq!(printer.error_count() != 0, expect_error);

        ctx
    }

    #[test]
    fn test_parse_stdout_invalid_command() {
        let _ = test_parse_stdout(&"PFX: XXXX FOO=baz", &"PFX: ", true, true);
    }

    #[test]
    fn test_parse_stdout_not_a_command() {
        let _ = test_parse_stdout(&"SET FOO=baz", &"PFX: ", false, false);
    }

    #[test]
    fn test_parse_stdout_set_overwrite_ok() {
        let ctx = test_parse_stdout(&"PFX: SET FOO=baz", &"PFX: ", true, false);

        assert_eq!(ctx.get("FOO"), Some("baz".to_string()));
    }

    #[test]
    fn test_parse_stdout_quoted_set_overwrite_ok() {
        let ctx = test_parse_stdout(&"PFX: SET \"FOO\"=\"baz\"", &"PFX: ", true, false);

        assert_eq!(ctx.get("FOO"), Some("baz".to_string()));
    }

    #[test]
    fn test_parse_stdout_set_add_ok() {
        let ctx = test_parse_stdout(&"PFX: SET BAZ=baz", &"PFX: ", true, false);

        assert_eq!(ctx.get("BAZ"), Some("baz".to_string()));
    }

    #[test]
    fn test_parse_stdout_quoted_set_add_ok() {
        let ctx = test_parse_stdout(&"PFX: SET \"BAZ\"=\"baz\"", &"PFX: ", true, false);

        assert_eq!(ctx.get("BAZ"), Some("baz".to_string()));
    }

    #[test]
    fn test_parse_stdout_set_invalid_name() {
        let ctx = test_parse_stdout(&"PFX: SET foo=baz", &"PFX: ", true, true);

        assert_eq!(ctx.get("FOO"), Some("bar".to_string()));
    }

    #[test]
    fn test_parse_stdout_set_no_equal() {
        let _ = test_parse_stdout(&"PFX: SET FOOBAR", &"PFX: ", true, true);
    }
}
