// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use cli::printer::Printer;

use contained_command::{Binding, Nspawn};

// - Constants:
// ----------------------------------------------------------------------

const DEFAULT_MACHINE_ID: [u8; 32] = [
    b'0', b'b', b'f', b'9', b'5', b'b', b'b', b'7', b'7', b'1', b'3', b'6', b'4', b'e', b'f', b'9',
    b'9', b'7', b'e', b'1', b'd', b'f', b'5', b'e', b'b', b'3', b'b', b'2', b'6', b'4', b'2', b'2',
];

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long = "log-level", default_value = "warn")]
    /// Set the log level
    log_level: cli::printer::LogLevel,

    #[arg(long = "agent-mode")]
    /// Turn on agent mode: FOR INTERNAL USE
    is_agent: bool,

    #[arg(long = "work-directory", short, default_value = ".")]
    work_directory: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let printer = Printer::new(&args.log_level);

    printer.h1("Setup", true);

    printer.trace("Logging is set up and ready.");
    printer.debug(&format!("Command line arguments: {args:?}"));

    if args.is_agent {
        printer.h1("Agent Mode", true);
        return cli::agent::run("Foobar");
    }

    printer.h1("Test Command", true);

    let root_directory = mktemp::Temp::new_dir_in(args.work_directory)
        .context("Failed to create temporary root filesystem folder")?;

    let exe_path = std::env::current_exe().context("Failed to find current executable path")?;

    let runner = Nspawn::default_runner(&root_directory)?
        .machine_id(DEFAULT_MACHINE_ID)
        .share_users()
        .binding(Binding::ro(&exe_path, &PathBuf::from("/agent")));

    let mut command = contained_command::Command::new("/agent");
    command.arg("--agent-mode");

    runner
        .run(
            &command,
            &|m| printer.trace(m),
            &|m| printer.error(m),
            &|m| printer.print_stdout(m),
            &|m| printer.print_stderr(m),
        )
        .await
        .context("Failed to containerize")?;

    Ok(())
}
