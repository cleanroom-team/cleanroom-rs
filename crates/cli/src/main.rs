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
    #[arg(long = "work-directory", short, default_value = "./work")]
    work_directory: PathBuf,

    /// The directory to store the final artifacts into
    #[arg(long = "artifact-directory", short, default_value = ".")]
    artifact_directory: PathBuf,

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

    let root_directory = args.work_directory.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir(&root_directory)
        .context("Failed to create root directory in work directory")?;
    let root_directory = root_directory
        .canonicalize()
        .context("Failed to canonicalize {root_directory:?}")?;
    printer.debug(&format!("ROOT_DIR: {root_directory:?}"));

    let exe_path = std::env::current_exe().context("Failed to find current executable path")?;
    printer.debug("own path: {exe_path:?}");

    printer.h2("system context", true);

    let mut ctx = cli::context::Context::default();

    printer.h1("Run agent", true);
    cli::agent_runner::run_agent(&printer, &root_directory, &exe_path, &mut ctx)
        .await
        .context("Failed to drive agent")?;
    Ok(())
}
