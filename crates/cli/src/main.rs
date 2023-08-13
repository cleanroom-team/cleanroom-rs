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

    /// The command to run
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Args, Debug)]
struct AgentMode {
    #[arg(long = "command-prefix", short)]
    command_prefix: String,
}

#[derive(Subcommand, Debug)]
#[command()]
enum Commands {
    Agent(AgentMode),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();

    if let Some(Commands::Agent(agent)) = args.command {
        return cli::agent::run(&agent.command_prefix);
    }

    let printer = Printer::new(&args.log_level, true);

    printer.h1("Setup", true);
    printer.h2("Bootstrap", true);

    printer.trace("Logging is set up and ready.");
    printer.debug(&format!("Command line arguments: {args:?}"));

    let myself = std::env::current_exe().context("Failed to find current executable path")?;

    printer.h2("system context", true);
    let mut base_ctx = {
        let mut builder = cli::context::ContextBuilder::new(printer);
        if let Some(ts) = &args.timestamp {
            builder = builder.timestamp(ts.clone())?;
        }
        if let Some(v) = &args.artifact_version {
            builder = builder.version(v.clone())?;
        }

        builder.build()
    };

    let printer = base_ctx.printer();

    base_ctx.command_manager_builder().scan_for_commands(
        &PathBuf::from("./commands"),
        false,
        &printer,
    )?;

    let mut ctx = base_ctx
        .set_system(
            "test_system",
            &args.work_directory,
            &args.artifact_directory,
            &args.busybox_binary,
            &myself,
        )
        .context("Failed to set up system context")?;

    printer.debug(&format!("{ctx}"));

    printer.h1("Run agent", true);
    cli::agent_runner::run_agent(&cli::Phases::Test, &mut ctx)
        .await
        .context("Failed to drive agent")?;
    Ok(())
}
