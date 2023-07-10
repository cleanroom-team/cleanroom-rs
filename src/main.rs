// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{env, path::PathBuf};

use clap::Parser;

mod commands;
mod printer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long = "log-level", default_value = "warn")]
    /// Set the log level
    log_level: printer::LogLevel,
    #[arg(long, default_values_t=[ "{EXE_DIR}/../../commands".to_string(), "{EXE_DIR}/../lib/cleanroom/commands".to_string(), "{EXE_DIR}/commads".to_string(), "{SYSTEMS_DIR}/cleanroom/commands".to_string() ])]
    /// The work area to create temporary artifacts in
    command_paths: Vec<String>,
    #[arg(long, default_value = ".")]
    /// The work area to create temporary artifacts in
    systems_directory: PathBuf,
    #[arg(long, default_value = "./work")]
    /// The work area to create temporary artifacts in
    work_directory: PathBuf,
    #[arg(long)]
    /// Clear the work-directory when the program exits cleanly
    clear_work_directory: bool,
    /// The systems to create
    systems: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let printer = printer::Printer::new(&args.log_level);

    printer.h1("Setup", true);
    printer.h2("Basic", true);

    printer.trace("Logging is set up and ready.");
    printer.debug(&format!("Comand line arguments: {args:?}"));

    let current_executable = env::current_exe()?;
    printer.debug(&format!("Current executable: {current_executable:?}"));

    let current_directory = env::current_dir()?;
    printer.debug(&format!("Current directory: {current_directory:?}"));

    let commands = commands::find_commands(
        &printer,
        &args.command_paths,
        &current_executable,
        &args.systems_directory,
        &current_directory,
    );

    Ok(())
}
