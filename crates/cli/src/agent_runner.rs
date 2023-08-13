// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::context::SystemContext;

use anyhow::Context;
use contained_command::{Binding, Command, Nspawn};

use std::path::PathBuf;

// - Constants:
// ----------------------------------------------------------------------

const DEFAULT_MACHINE_ID: [u8; 32] = [
    b'0', b'b', b'f', b'9', b'5', b'b', b'b', b'7', b'7', b'1', b'3', b'6', b'4', b'e', b'f', b'9',
    b'9', b'7', b'e', b'1', b'd', b'f', b'5', b'e', b'b', b'3', b'b', b'2', b'6', b'4', b'2', b'2',
];

fn parse_stdout(m: &str, command_prefix: &str, ctx: &mut SystemContext) -> bool {
    let p = ctx.printer();
    let Some(cmd) = m.strip_prefix(command_prefix) else {
        return false;
    };

    p.trace(&format!("Processing {}", cmd));
    if let Some(to_set) = cmd.strip_prefix("SET ") {
        if let Some((k, v)) = to_set.split_once('=') {
            let k = k.trim_matches('"');
            let v = v.trim_matches('"');
            if ctx.set(k, v, false, false).is_err() {
                p.error(&format!(
                    "Could not parse arguments after SET: {k:?} is not a valid variable name"
                ));
            }
        } else {
            p.error(&format!(
                "Could not parse arguments after SET: No '=' found in {to_set:?}"
            ));
        }
    } else if let Some(to_set) = cmd.strip_prefix("SET_RO ") {
        if let Some((k, v)) = to_set.split_once('=') {
            let k = k.trim_matches('"');
            let v = v.trim_matches('"');
            if ctx.set(k, v, true, false).is_err() {
                p.error(&format!(
                    "Could not parse arguments after SET_RO: {k:?} is not a valid variable name"
                ));
            }
        } else {
            p.error(&format!(
                "Could not parse arguments after SET: No '=' found in {to_set:?}"
            ));
        }
    } else {
        p.error(&format!("Agent asked to process unknown command {cmd:?}"))
    }
    true
}

#[allow(clippy::needless_pass_by_ref_mut)] // FIXME: It's not useless: It's passed on to parse_stdout!
pub async fn run_agent(phase: &crate::Phases, ctx: &mut SystemContext) -> anyhow::Result<()> {
    let p = ctx.printer();
    p.h1("Run Agent", true);

    let Some(phase_script) =
        crate::scripts::create_script(phase, ctx).context("Failed to create phase script")?
    else {
        return Ok(());
    };

    p.h2("Run in container", true);
    let runner = Nspawn::default_runner(&ctx.root_directory().unwrap())?
        .machine_id(DEFAULT_MACHINE_ID)
        .binding(Binding::ro(
            &phase_script,
            &PathBuf::from("/clrm/script.sh"),
        ))
        .binding(Binding::ro(
            &ctx.my_binary().unwrap(),
            &PathBuf::from("/clrm/agent"),
        ))
        .binding(Binding::ro(
            &ctx.busybox_binary().unwrap(),
            &PathBuf::from("/clrm/busybox"),
        ))
        .binding(Binding::ro(
            &phase_script,
            &PathBuf::from("/clrm/script.sh"),
        ));

    let command_prefix = uuid::Uuid::new_v4().to_string();
    let command = {
        let mut command = Command::new("/clrm/agent");
        command.arg("agent");
        command.arg(&format!("--command-prefix={command_prefix}"));
        command
    };

    let command_prefix = format!("{command_prefix}: ");
    runner
        .run(
            &command,
            &|m| p.trace(m),
            &|m| p.error(m),
            &mut |m| {
                if !parse_stdout(m, &command_prefix, ctx) {
                    p.print_stdout(m);
                }
            },
            &mut |m| p.print_stderr(m),
        )
        .await
        .context("Failed to containerize")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::printer::{LogLevel, Printer};

    use super::*;

    fn test_parse_stdout(
        input: &str,
        command_prefix: &str,
        expect_handled: bool,
        expect_error: bool,
    ) -> crate::context::SystemContext {
        let printer = Printer::new(&LogLevel::Off, false);

        let ctx = crate::context::ContextBuilder::new(printer).build();
        let mut ctx = ctx.test_system();

        ctx.set("FOO", "bar", false, false).unwrap();
        ctx.set("BAR", "foo", false, false).unwrap();

        assert_eq!(
            parse_stdout(input, command_prefix, &mut ctx),
            expect_handled
        );

        assert_eq!(ctx.printer().error_count() != 0, expect_error);

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
