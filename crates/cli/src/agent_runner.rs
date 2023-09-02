// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{context::RunContext, Phases};

use anyhow::Context;
use contained_command::{Binding, Command, Nspawn, RunEnvironment, Runner};

use std::{ffi::OsString, path::PathBuf};

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
    } else if let Some(status) = cmd.strip_prefix("PUSH ") {
        let status = status.trim().trim_matches('"');
        ctx.printer().push_status(status);
    } else if cmd == "POP" {
        ctx.printer().pop_status();
    } else {
        p.error(&format!("Agent asked to process unknown command {cmd:?}"))
    }
    true
}

fn run_in_bootstrap(phase: &Phases) -> bool {
    phase == &Phases::Install || phase == &Phases::BuildArtifacts || phase == &Phases::TestArtifacts
}

fn mount_artifacts_directory(phase: &Phases) -> bool {
    phase == &Phases::BuildArtifacts || phase == &Phases::TestArtifacts
}
fn mount_root_fs(phase: &Phases) -> bool {
    phase != &Phases::TestArtifacts
}

fn create_runner(
    ctx: &RunContext,
    command: &str,
    phase: &Phases,
    extra_bindings: &[String],
) -> anyhow::Result<Runner<contained_command::Nspawn>> {
    let p = ctx.printer();

    p.h2(&format!("Create \"{phase}\""), true);
    let agent_script =
        crate::scripts::create_script(ctx, command).context("Failed to create agent script")?;

    let mut flags = vec![];

    let mut runner = if run_in_bootstrap(phase) {
        p.info(&format!("Running \"{phase}\" [BOOTSTRAP]"));
        flags.push("BOOTSTRAP");
        let mut runner = Nspawn::default_runner(ctx.bootstrap_environment().clone())?
            .env("CLRM_CONTAINER", "bootstrap");

        if mount_root_fs(phase) {
            runner = runner
                .binding(Binding::rw(
                    &ctx.root_directory().unwrap(),
                    &PathBuf::from("/tmp/clrm/root_fs"),
                ))
                .env("ROOT_FS", "/tmp/clrm/root_fs")
        }
        if mount_artifacts_directory(phase) {
            runner = runner
                .binding(Binding::rw(
                    &ctx.artifacts_directory().unwrap(),
                    &PathBuf::from("/tmp/clrm/artifacts_fs"),
                ))
                .env("ARTIFACTS_FS", "/tmp/clrm/artifacts_fs")
        }

        runner
    } else {
        p.debug(&format!("Running \"{phase}\" [ROOT]"));
        flags.push("ROOT");
        Nspawn::default_runner(RunEnvironment::Directory(
            ctx.root_directory().unwrap().clone(),
        ))?
        // .binding(Binding::tmpfs(&PathBuf::from("/tmp")))
        .env("CLRM_CONTAINER", "root_fs")
        .env("ROOT_FS", "/")
        .persistent_root()
    };

    runner = if ctx.wants_network(phase) {
        flags.push("NET");
        runner.with_network().env("PHASE_IS_NETWORKED", "1")
    } else {
        flags.push("no net");
        runner.env("PHASE_IS_NETWORKED", "0")
    };

    runner = runner
        .machine_id(DEFAULT_MACHINE_ID)
        .share_users()
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
        ));

    for extra in extra_bindings {
        let binding =
            Binding::try_from(extra.as_str()).context("Failed to apply extra arguments")?;
        runner = runner.binding(binding);
    }

    p.h2(&format!("Running \"{phase}\" [{}]", flags.join(", ")), true);

    Ok(runner)
}

#[allow(clippy::needless_pass_by_ref_mut)] // FIXME: It's not useless: It's passed on to parse_stdout!
pub async fn enter_agent_phase(
    ctx: &mut RunContext,
    command: &str,
    phase: &Phases,
    extra_bindings: &[String],
) -> anyhow::Result<()> {
    let p = ctx.printer();
    p.h2("Enter container in phase \"{phase}\"", false);
    let runner = create_runner(ctx, command, phase, extra_bindings)?.with_network();

    let command = {
        let mut command = Command::new("/tmp/clrm/busybox");
        command.arg("sh");
        command.env("CURRENT_PHASE", phase.to_string());
        command
    };

    let (mut child, command_path, args) = runner
        .run_raw(&command, false)
        .context("Failed to containerize")?;

    println!(
        "\nRunning: {command_path:?} {}",
        args.join(&OsString::from(" ")).to_string_lossy()
    );

    let result = child.wait().await?;

    if result.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Container was not terminated successfully"))
    }
}

#[allow(clippy::needless_pass_by_ref_mut)] // FIXME: It's not useless: It's passed on to parse_stdout!
pub async fn run_agent_phase(
    ctx: &mut RunContext,
    command: &str,
    phase: &Phases,
    extra_bindings: &[String],
) -> anyhow::Result<()> {
    let p = ctx.printer();
    let runner = create_runner(ctx, command, phase, extra_bindings)?;

    let command_prefix = uuid::Uuid::new_v4().to_string();
    let command = {
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
    extra_bindings: &[String],
) -> anyhow::Result<()> {
    let p = ctx.printer();
    p.h1("Run Agent", true);

    for phase in Phases::iter() {
        if ctx.check_debug_option(&crate::DebugOptions::PrintRunContext) {
            p.debug(&format!("RunContext in {phase} is:\n{ctx}"));
        }
        if enter_phase.as_ref() == Some(phase) {
            return enter_agent_phase(ctx, command, phase, extra_bindings).await;
        } else {
            run_agent_phase(ctx, command, phase, extra_bindings).await?;
        }
    }

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
    ) -> crate::context::RunContext {
        let ctx = crate::context::ContextBuilder::default().build();
        let mut ctx = ctx.test_system(&LogLevel::Off);

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
