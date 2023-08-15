// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};

use cli::printer::Printer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    #[arg(long = "log-level", default_value = "warn")]
    /// Set the log level
    log_level: cli::printer::LogLevel,

    /// The command to run
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug)]
struct AgentMode {
    /// The prefix used to send commands to the agent runner
    #[arg(long = "command-prefix", short)]
    command_prefix: String,
    /// The phase to run
    phase: cli::Phases,
}

#[derive(Args, Debug)]
struct RunMode {
    /// The directory to create temporary files in
    #[arg(long = "work-directory", default_value = "./work")]
    work_directory: PathBuf,

    /// The directory to store the final artifacts into
    #[arg(long = "artifact-directory", default_value = ".")]
    artifact_directory: PathBuf,

    /// The directory to store the final artifacts into (defaults to current time if unset)
    #[arg(long = "timestamp")]
    timestamp: Option<String>,

    /// The version string to use (defaults to timestamp if unset)
    #[arg(long = "artifact-version")]
    artifact_version: Option<String>,

    /// The version string to use (defaults to timestamp if unset)
    #[arg(long = "busybox-binary", default_value = "/usr/bin/busybox")]
    busybox_binary: PathBuf,

    /// The commands to run
    command: String,
}

#[derive(Subcommand, Debug)]
#[command()]
enum Commands {
    /// Run as an agent inside a container. For internal use
    Agent(AgentMode),
    /// Run some command
    Run(RunMode),
}

fn create_system_context(
    printer: Printer,
    run: &RunMode,
) -> anyhow::Result<cli::context::RunContext> {
    let myself = std::env::current_exe().context("Failed to find current executable path")?;

    printer.h2("system context", true);
    let mut base_ctx = {
        let mut builder = cli::context::ContextBuilder::new(printer);
        if let Some(ts) = &run.timestamp {
            builder = builder.timestamp(ts.clone())?;
        }
        if let Some(v) = &run.artifact_version {
            builder = builder.version(v.clone())?;
        }

        builder.build()
    };

    let printer = base_ctx.printer();

    base_ctx
        .command_manager_builder()
        .scan_for_commands(&PathBuf::from("."), false, &printer)?;

    let ctx = base_ctx
        .set_system(
            &run.work_directory,
            &run.artifact_directory,
            &run.busybox_binary,
            &myself,
        )
        .context("Failed to set up system context")?;

    Ok(ctx)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();

    match &args.command {
        Commands::Agent(agent) => return cli::agent::run(&agent.command_prefix, &agent.phase),
        Commands::Run(run) => {
            let printer = Printer::new(&args.log_level, true);

            printer.h1("Setup", true);
            printer.h2("Bootstrap", true);

            printer.trace("Logging is set up and ready.");
            printer.debug(&format!("Command line arguments: {args:?}"));

            let mut ctx =
                create_system_context(printer, run).context("Failed to create system context")?;

            let printer = ctx.printer();
            printer.h1("Run agent", true);
            cli::agent_runner::run_agent(&mut ctx, &run.command)
                .await
                .context("Failed to drive agent")?;
            Ok(())
        }
    }
}
