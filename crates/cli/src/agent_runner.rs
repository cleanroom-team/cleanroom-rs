// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{context::RunContext, Phases};

use anyhow::Context;
use contained_command::{Binding, Command, Nspawn, RunEnvironment};

use std::path::PathBuf;

// - Constants:
// ----------------------------------------------------------------------

const DEFAULT_MACHINE_ID: [u8; 32] = [
    b'0', b'b', b'f', b'9', b'5', b'b', b'b', b'7', b'7', b'1', b'3', b'6', b'4', b'e', b'f', b'9',
    b'9', b'7', b'e', b'1', b'd', b'f', b'5', b'e', b'b', b'3', b'b', b'2', b'6', b'4', b'2', b'2',
];

fn parse_stdout(m: &str, command_prefix: &str, ctx: &mut RunContext) -> bool {
    let p = ctx.printer();
    let Some(cmd) = m.strip_prefix(command_prefix) else {
        return false;
    };

    p.trace(&format!("Processing {}", cmd));
    if let Some(to_set) = cmd.strip_prefix("SET ") {
        if let Some((k, v)) = to_set.split_once('=') {
            let k = k.trim().trim_matches('"');
            let v = v.trim().trim_matches('"');
            if let Err(e) = ctx.set(k, v, false, false) {
                p.error(&format!("Could not parse arguments after SET {k}: {e}"));
            }
        } else {
            p.error(&format!(
                "Could not parse arguments after SET: No '=' found in {to_set:?}"
            ));
        }
    } else if let Some(to_set) = cmd.strip_prefix("SET_RO ") {
        if let Some((k, v)) = to_set.split_once('=') {
            let k = k.trim().trim_matches('"');
            let v = v.trim().trim_matches('"');
            if let Err(e) = ctx.set(k, v, true, false) {
                p.error(&format!("Could not parse arguments after SET_RO {k}: {e}"));
            }
        } else {
            p.error(&format!(
                "Could not parse arguments after SET: No '=' found in {to_set:?}"
            ));
        }
    } else if let Some(status) = cmd.strip_prefix("STATUS ") {
        let status = status.trim().trim_matches('"');
        ctx.printer().h3(status, true);
    } else {
        p.error(&format!("Agent asked to process unknown command {cmd:?}"))
    }
    true
}

fn run_in_bootstrap(phase: &Phases) -> bool {
    phase == &Phases::Install || phase == &Phases::BuildArtifacts || phase == &Phases::TestArtifacts
}

#[allow(clippy::needless_pass_by_ref_mut)] // FIXME: It's not useless: It's passed on to parse_stdout!
pub async fn run_agent_phase(
    ctx: &mut RunContext,
    command: &str,
    phase: &Phases,
    enter_phase: &Option<Phases>,
) -> anyhow::Result<()> {
    let p = ctx.printer();
    p.debug(&format!("Entering {phase} with {ctx}"));
    let agent_script =
        crate::scripts::create_script(ctx, command).context("Failed to create agent script")?;

    p.h2("Run in container", true);
    let runner = {
        let runner = if run_in_bootstrap(phase) {
            Nspawn::default_runner(ctx.bootstrap_environment().clone())?
                // .binding(Binding::tmpfs(&PathBuf::from("/tmp")))
                .binding(Binding::rw(
                    &ctx.root_directory().unwrap(),
                    &PathBuf::from("/tmp/clrm/root_fs"),
                ))
                .env("CLRM_CONTAINER", "bootstrap")
                .env("ROOT_FS", "/tmp/clrm/root_fs")
        } else {
            Nspawn::default_runner(RunEnvironment::Directory(
                ctx.root_directory().unwrap().clone(),
            ))?
            // .binding(Binding::tmpfs(&PathBuf::from("/tmp")))
            .env("CLRM_CONTAINER", "root_fs")
            .env("ROOT_FS", "/")
        };

        runner
            .machine_id(DEFAULT_MACHINE_ID)
            .binding(Binding::ro(
                &ctx.my_binary().unwrap(),
                &PathBuf::from("/tmp/clrm/agent"),
            ))
            .binding(Binding::ro(
                &ctx.busybox_binary().unwrap(),
                &PathBuf::from("/tmp/clrm/busybox"),
            ))
            .binding(Binding::ro(
                &agent_script,
                &PathBuf::from("/tmp/clrm/script.sh"),
            ))
    };

    let command_prefix = uuid::Uuid::new_v4().to_string();
    let command = if Some(phase) == enter_phase.as_ref() {
        let mut command = Command::new("/tmp/clrm/busybox");
        command.arg("sh");
        command
    } else {
        let mut command = Command::new("/tmp/clrm/agent");
        command.arg("agent");
        command.arg(&format!("--command-prefix={command_prefix}"));
        command.arg(&phase.to_string());
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

#[allow(clippy::needless_pass_by_ref_mut)] // FIXME: It's not useless: It's passed on to run_agent_phase
pub async fn run_agent(
    ctx: &mut RunContext,
    command: &str,
    enter_phase: &Option<Phases>,
) -> anyhow::Result<()> {
    let p = ctx.printer();
    p.h1("Run Agent", true);

    for phase in Phases::iter() {
        run_agent_phase(ctx, command, phase, enter_phase).await?;
    }

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
    ) -> crate::context::RunContext {
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
