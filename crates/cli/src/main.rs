// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{path::PathBuf, rc::Rc};

use anyhow::Context;
use clap::{Args, Parser, Subcommand};

use cli::{printer::Printer, DebugOptions};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    #[arg(long, default_value = "warn")]
    /// Set the log level
    log_level: cli::printer::LogLevel,

    /// The command to run
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug)]
struct ExtraCommandPath {
    /// Extends the default lookup path for commands. Later directories can
    /// overwrite commands in earlier directories.
    #[arg(long, env = "CLRM_EXTRA_COMMAND_PATH", value_delimiter = ':')]
    extra_command_path: Option<Vec<PathBuf>>,
}

impl std::ops::Deref for ExtraCommandPath {
    type Target = Vec<PathBuf>;

    fn deref(&self) -> &Self::Target {
        static EMPTY: Vec<PathBuf> = Vec::new();
        self.extra_command_path.as_ref().unwrap_or(&EMPTY)
    }
}

#[derive(Args, Debug)]
struct BuildAgentCommand {
    /// The prefix used to send commands to the agent runner
    #[arg(long, short)]
    command_prefix: String,
    /// The phase to run
    phase: cli::Phases,
}

#[derive(Args, Debug)]
struct CommandListCommand {
    /// Print more information
    #[arg(long)]
    verbose: bool,

    #[command(flatten)]
    extra_command_path: ExtraCommandPath,
}

#[derive(Args, Debug)]
struct DumpCommand {
    /// The command to dump
    name: String,

    #[command(flatten)]
    extra_command_path: ExtraCommandPath,
}

#[derive(Args, Debug)]
struct InitializeCommand {
    /// The busybox binary to use
    #[arg(long, short)]
    busybox_binary: Option<PathBuf>,
    /// The base distribution this configuration is supposed to cover
    #[arg(long, short, default_value = "arch")]
    distribution: cli::Distributions,
    /// The directory to initialize
    #[arg(default_value = ".")]
    directory: PathBuf,
}

#[derive(Args, Debug)]
struct BuildCommand {
    /// The directory to create temporary files in
    #[arg(long, default_value = "./work", env = "CLRM_WORK_DIR")]
    work_directory: PathBuf,

    /// The directory to store the final artifacts into
    #[arg(long, default_value = ".", env = "CLRM_ARTIFACTS_DIR")]
    artifacts_directory: PathBuf,

    /// The current time -- used as a version if nothing else is specified
    #[arg(long)]
    timestamp: Option<String>,

    /// The version string to use (defaults to timestamp if unset)
    #[arg(long)]
    artifact_version: Option<String>,

    /// The busybox binary to use
    #[arg(long, default_value = "/usr/bin/busybox", env = "CLRM_BUSYBOX")]
    busybox_binary: PathBuf,

    /// A disk image to use as a bootstrap environment (conflicts with --bootstrap-directory)
    #[arg(
        long,
        conflicts_with = "bootstrap_directory",
        env = "CLRM_BOOTSTRAP_IMAGE"
    )]
    bootstrap_image: Option<PathBuf>,

    /// A bootstrap environment installed into a directory (conflicts with --bootstrap-image)
    #[arg(long, env = "CLRM_BOOTSTRAP_DIR")]
    bootstrap_directory: Option<PathBuf>,

    /// Enter a debug environment in the provided phase
    #[arg(long, env = "CLRM_EXTRA_BINDINGS", value_delimiter = ',')]
    extra_bindings: Vec<String>,

    #[command(flatten)]
    extra_command_path: ExtraCommandPath,

    /// Enter a debug environment in the provided phase
    #[arg(long, env = "CLRM_NETWORKED_PHASES", value_delimiter = ',')]
    networked_phases: Vec<cli::Phases>,

    /// Enter a debug environment in the provided phase
    #[arg(long)]
    enter_phase: Option<cli::Phases>,

    /// The commands to run
    command: String,

    #[arg(long, env = "CLRM_TRACE_SCRIPT", value_delimiter = ',', hide = true)]
    debug_options: Option<Vec<cli::DebugOptions>>,
}

#[allow(clippy::large_enum_variant)] // This is used exactly once, a bit of wasted space is fine
#[derive(Subcommand, Debug)]
#[command()]
enum Commands {
    /// Run as an agent inside a container. For internal use
    #[command(hide = true)]
    BuildAgent(BuildAgentCommand),
    /// Print a list of known commands
    CommandList(CommandListCommand),
    /// Dump a command definition to stdout
    DumpCommand(DumpCommand),
    /// Initialize a directory to hold a cleanroom configuration
    Initialize(InitializeCommand),
    /// Run some command
    Build(BuildCommand),
}

fn create_command_manager(
    extra_command_path: &[PathBuf],
) -> anyhow::Result<cli::commands::CommandManager> {
    let mut builder = cli::commands::CommandManagerBuilder::default();
    for command_path in extra_command_path {
        builder
            .scan_for_commands(command_path)
            .context(format!("Failed to find command in {command_path:?}"))?;
    }

    Ok(builder.build())
}

fn create_build_context(
    printer: Printer,
    run: &BuildCommand,
) -> anyhow::Result<cli::context::BuildContext> {
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

    let debug_options = {
        static DEFAULT: Vec<DebugOptions> = vec![];
        run.debug_options.as_ref().unwrap_or(&DEFAULT)
    };

    let ctx = base_ctx
        .create_run_context(
            create_command_manager(&run.extra_command_path)?,
            Rc::new(printer),
            &run.work_directory,
            &run.artifacts_directory,
            &run.busybox_binary,
            &myself,
            bootstrap_environment,
            &run.networked_phases,
            debug_options,
        )
        .context("Failed to set up system context")?;

    Ok(ctx)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();

    match &args.command {
        Commands::BuildAgent(agent) => cli::agent::run(&agent.command_prefix, &agent.phase),
        Commands::CommandList(list) => {
            let command_manager = create_command_manager(&list.extra_command_path)?;
            println!("{}", command_manager.list_commands(list.verbose));
            Ok(())
        }
        Commands::DumpCommand(dc) => {
            let command_manager = create_command_manager(&dc.extra_command_path)?;
            let cmd = command_manager.command(&dc.name)?;
            println!("{}", cmd.dump_source());
            Ok(())
        }
        Commands::Initialize(init) => {
            cli::init::initialize(&init.busybox_binary, &init.distribution, &init.directory)
        }
        Commands::Build(build) => {
            let printer = Printer::new(&args.log_level, true);

            printer.h1("Setup", true);
            printer.h2("Bootstrap", true);

            printer.trace("Logging is set up and ready.");
            printer.debug(&format!("Command line arguments: {args:?}"));

            let mut ctx =
                create_build_context(printer, build).context("Failed to create system context")?;

            let printer = ctx.printer();
            printer.h1("Run agent", true);
            cli::agent_runner::run_build_agent(
                &mut ctx,
                &build.command,
                &build.enter_phase,
                &build.extra_bindings,
            )
            .await
            .context("Failed to drive agent")?;
            Ok(())
        }
    }
}
