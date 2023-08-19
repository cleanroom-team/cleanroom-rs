// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{path::PathBuf, rc::Rc};

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

    /// The busybox binary to use
    #[arg(long = "busybox-binary", default_value = "/usr/bin/busybox")]
    busybox_binary: PathBuf,

    /// A disk image to use as a bootstrap environment (conflicts with --bootstrap-directory)
    #[arg(long = "bootstrap-image", conflicts_with = "bootstrap_directory")]
    bootstrap_image: Option<PathBuf>,

    /// A bootstrap environment installed into a directory (conflicts with --bootstrap-image)
    #[arg(long = "bootstrap-directory")]
    bootstrap_directory: Option<PathBuf>,

    /// Enter a debug environment in the provided phase
    #[arg(long = "enter-phase")]
    enter_phase: Option<cli::Phases>,

    /// The commands to run
    command: String,
}

#[derive(Args, Debug)]
struct CommandListMode {
    /// Print more information
    #[arg(long = "verbose")]
    verbose: bool,
}

#[derive(Args, Debug)]
struct DumpCommand {
    /// The command to dump
    name: String,
}

#[derive(Subcommand, Debug)]
#[command()]
enum Commands {
    /// Run as an agent inside a container. For internal use
    Agent(AgentMode),
    /// Run some command
    Run(RunMode),
    /// Print a list of known commands
    CommandList(CommandListMode),
    /// Dump a command definition to stdout
    DumpCommand(DumpCommand),
}

fn create_command_manager() -> anyhow::Result<cli::commands::CommandManager> {
    let mut builder = cli::commands::CommandManagerBuilder::default();
    builder
        .scan_for_commands(&PathBuf::from("."))
        .context("Failed to find command in `.`")?;

    Ok(builder.build())
}

fn create_run_context(printer: Printer, run: &RunMode) -> anyhow::Result<cli::context::RunContext> {
    let base_ctx = {
        let mut builder = cli::context::ContextBuilder::default();
        if let Some(ts) = &run.timestamp {
            builder = builder.timestamp(ts.clone())?;
        }
        if let Some(v) = &run.artifact_version {
            builder = builder.version(v.clone())?;
        }

        builder.build()
    };

    let myself = std::env::current_exe().context("Failed to find current executable path")?;

    let bootstrap_environment =
        cli::RunEnvironment::new(&run.bootstrap_directory, &run.bootstrap_image)?;

    let ctx = base_ctx
        .create_run_context(
            create_command_manager()?,
            Rc::new(printer),
            &run.work_directory,
            &run.artifact_directory,
            &run.busybox_binary,
            &myself,
            bootstrap_environment,
        )
        .context("Failed to set up system context")?;

    Ok(ctx)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();

    match &args.command {
        Commands::Agent(agent) => cli::agent::run(&agent.command_prefix, &agent.phase),
        Commands::CommandList(list) => {
            let command_manager = create_command_manager()?;
            println!("{}", command_manager.list_commands(list.verbose));
            Ok(())
        }
        Commands::DumpCommand(dc) => {
            let command_manager = create_command_manager()?;
            let cmd = command_manager.command(&dc.name)?;
            println!("{}", cmd.dump_source());
            Ok(())
        }
        Commands::Run(run) => {
            let printer = Printer::new(&args.log_level, true);

            printer.h1("Setup", true);
            printer.h2("Bootstrap", true);

            printer.trace("Logging is set up and ready.");
            printer.debug(&format!("Command line arguments: {args:?}"));

            let mut ctx =
                create_run_context(printer, run).context("Failed to create system context")?;

            let printer = ctx.printer();
            printer.h1("Run agent", true);
            cli::agent_runner::run_agent(&mut ctx, &run.command, &run.enter_phase)
                .await
                .context("Failed to drive agent")?;
            Ok(())
        }
    }
}
