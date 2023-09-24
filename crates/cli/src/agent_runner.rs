// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    commands::{CommandName, VariableName},
    context::BuildContext,
    Phases,
};

use anyhow::{anyhow, Context};
use contained_command::{Binding, Command, Nspawn, RunEnvironment, Runner};

use std::{ffi::OsString, path::PathBuf};

// - Constants:
// ----------------------------------------------------------------------

const DEFAULT_MACHINE_ID: [u8; 32] = [
    b'0', b'b', b'f', b'9', b'5', b'b', b'b', b'7', b'7', b'1', b'3', b'6', b'4', b'e', b'f', b'9',
    b'9', b'7', b'e', b'1', b'd', b'f', b'5', b'e', b'b', b'3', b'b', b'2', b'6', b'4', b'2', b'2',
];

fn handle_set(cmd: &str, ctx: &mut BuildContext) -> anyhow::Result<bool> {
    let Some(to_set) = cmd.strip_prefix("SET ") else {
        return Ok(false);
    };

    let Some((k, v)) = to_set.split_once('=') else {
        return Err(anyhow!(format!(
            "Could not parse arguments after SET: No '=' found in {to_set:?}"
        )));
    };

    let k = k.trim().trim_matches('"');
    let v = v.trim().trim_matches('"');
    ctx.set(k, v, false, false)
        .context(format!("Failed fo SET {k} to {v}"))?;

    Ok(true)
}

fn handle_set_ro(cmd: &str, ctx: &mut BuildContext) -> anyhow::Result<bool> {
    let Some(to_set) = cmd.strip_prefix("SET_RO ") else {
        return Ok(false);
    };

    let Some((k, v)) = to_set.split_once('=') else {
        return Err(anyhow!(format!(
            "Could not parse arguments after SET: No '=' found in {to_set:?}"
        )));
    };

    let k = k.trim().trim_matches('"');
    let v = v.trim().trim_matches('"');
    ctx.set(k, v, true, false)
        .context(format!("Failed fo SET_RO {k} to {v}"))?;

    Ok(true)
}

fn handle_add_dependency(cmd: &str, ctx: &mut BuildContext) -> anyhow::Result<bool> {
    let Some(to_set) = cmd.strip_prefix("ADD_DEPENDENCY ") else {
        return Ok(false);
    };

    let Some((k, v)) = to_set.split_once('=') else {
        return Err(anyhow!(format!(
            "Could not parse arguments after ADD_DEPENDENCY: No '=' found in {to_set:?}"
        )));
    };

    let key = VariableName::try_from(k.trim().trim_matches('"').to_string())
        .context(format!("Could not convert {k} to variable name"))?;
    let value = CommandName::try_from(v.trim().trim_matches('"').to_string())
        .context(format!("Could not convert {v} to command name"))?;
    ctx.add_dependency(key.clone(), value.clone())
        .context(format!("Failed to ADD_DEPENDENCY {key} using {value}"))?;

    Ok(true)
}

fn parse_stdout(
    m: &str,
    command_prefix: &str,
    ctx: &mut BuildContext,
    current_status: &mut Option<crate::printer::Headline>,
) -> anyhow::Result<bool> {
    let p = ctx.printer();
    let Some(cmd) = m.strip_prefix(command_prefix) else {
        return Ok(false);
    };

    p.trace(&format!("Processing {}", cmd));
    if handle_set(cmd, ctx)? || handle_set_ro(cmd, ctx)? || handle_add_dependency(cmd, ctx)? {
        return Ok(true);
    }

    if let Some(status) = cmd.strip_prefix("STATUS ") {
        let status = status.trim().trim_matches('"');
        *current_status = Some(ctx.printer().push_headline(status, true));
    } else if let Some(status) = cmd.strip_prefix("PUSH ") {
        let status = status.trim().trim_matches('"');
        ctx.printer().push_status(status);
    } else if cmd == "POP" {
        ctx.printer().pop_status();
    } else {
        return Err(anyhow!(format!(
            "Agent asked to process unknown command {cmd:?}"
        )));
    }

    Ok(true)
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
    ctx: &BuildContext,
    command: &CommandName,
    phase: &Phases,
    extra_bindings: &[String],
) -> anyhow::Result<Runner<contained_command::Nspawn>> {
    let p = ctx.printer();
    let artifacts_fs = "/tmp/clrm/artifacts_fs";

    let _hl = p.push_headline(&format!("Create \"{phase}\""), true);
    let agent_script =
        crate::scripts::create_script(ctx, command).context("Failed to create agent script")?;

    let mut flags = vec![];

    let mut runner = if run_in_bootstrap(phase) {
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
        runner
    } else {
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

    if mount_artifacts_directory(phase) {
        flags.push("ARTIFACTS");
        runner = runner
            .binding(Binding::rw(
                &ctx.artifacts_directory().unwrap(),
                &PathBuf::from(artifacts_fs),
            ))
            .env("ARTIFACTS_FS", artifacts_fs)
    }

    Ok(runner.description(flags.join(", ").to_string()))
}

#[allow(clippy::needless_pass_by_ref_mut)] // FIXME: It's not useless: It's passed on to parse_stdout!
pub async fn enter_agent_phase(
    ctx: &mut BuildContext,
    command: &CommandName,
    phase: &Phases,
    extra_bindings: &[String],
) -> anyhow::Result<()> {
    let p = ctx.printer();
    let runner = create_runner(ctx, command, phase, extra_bindings)?.with_network();
    let _hl = p.push_headline(
        &format!("Enter container in \"{phase}\" [{}]", runner.describe()),
        false,
    );

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
        Err(anyhow!("Container was not terminated successfully"))
    }
}

#[allow(clippy::needless_pass_by_ref_mut)] // FIXME: It's not useless: It's passed on to parse_stdout!
pub async fn run_agent_phase(
    ctx: &mut BuildContext,
    command: &CommandName,
    phase: &Phases,
    extra_bindings: &[String],
) -> anyhow::Result<()> {
    let p = ctx.printer();
    let runner = create_runner(ctx, command, phase, extra_bindings)?;
    let _hl = p.push_headline(
        &format!("Running container in \"{phase}\" [{}]", runner.describe()),
        false,
    );

    let command_prefix = uuid::Uuid::new_v4().to_string();
    let command = {
        let mut command = Command::new("/tmp/clrm/agent");
        command.arg("build-agent");
        command.arg(&format!("--command-prefix={command_prefix}"));
        command.arg(&phase.to_string());
        command
    };

    let command_prefix = format!("{command_prefix}: ");
    {
        let mut current_status = None;
        runner
            .run(
                &command,
                &|m| p.trace(m),
                &|m| p.error(m),
                &mut |m| {
                    match parse_stdout(m, &command_prefix, ctx, &mut current_status) {
                        Ok(true) => {}
                        Ok(false) => p.print_stdout(m),
                        Err(e) => p.error(&format!("Failed to parse stdout: {e:?}")),
                    };
                },
                &mut |m| p.print_stderr(m),
            )
            .await
            .context("Failed to containerize")?;
    }

    Ok(())
}

#[allow(clippy::needless_pass_by_ref_mut)] // FIXME: It's not useless: It's passed on to run_agent_phase
pub async fn run_build_agent(
    ctx: &mut BuildContext,
    command: &CommandName,
    enter_phase: &Option<Phases>,
    extra_bindings: &[String],
) -> anyhow::Result<()> {
    let p = ctx.printer();
    let _hl = p.push_headline("Run Agent", true);

    for phase in Phases::iter() {
        if ctx.check_debug_option(&crate::DebugOptions::PrintBuildContext) {
            p.debug(&format!("RunContext in {phase} is:\n{ctx}"));
        }
        if enter_phase.as_ref() == Some(phase) {
            return enter_agent_phase(ctx, command, phase, extra_bindings).await;
        } else {
            run_agent_phase(ctx, command, phase, extra_bindings).await?;
            let dependencies = ctx.take_dependencies();

            for (name, command) in dependencies {
                println!("Dependencies after phase {phase}: {name} => {command}");
            }
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
    ) -> crate::context::BuildContext {
        let ctx = crate::context::ContextBuilder::default().build();
        let mut ctx = ctx.test_system(&LogLevel::Off);

        ctx.set("FOO", "bar", false, false).unwrap();
        ctx.set("BAR", "foo", false, false).unwrap();

        let mut current_headline = None;

        assert_eq!(
            parse_stdout(input, command_prefix, &mut ctx, &mut current_headline),
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
