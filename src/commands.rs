// Copyright © Tobias Hunger <tobias.hunger@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The commands known to `cleanroom`

use std::path::{Path, PathBuf};

use anyhow::Context;

/// A Command that can be used in the system definition
pub struct Command {}

pub type Commands = Vec<Command>;

pub fn find_commands(
    printer: &crate::printer::Printer,
    command_path: &[String],
    current_executable: &Path,
    systems_directory: &Path,
    current_directoy: &Path,
) -> anyhow::Result<Commands> {
    printer.h2("Finding commands", true);

    let command_paths = generate_full_command_path(
        printer,
        command_path,
        current_executable,
        systems_directory,
        current_directoy,
    )?;
    printer.debug(&format!("Expanded command path: {:?}", command_paths));

    command_paths.iter().flat_map(|_| {
       vec![Ok(Command {})]
    }).collect()
}

fn generate_full_command_path(
    printer: &crate::printer::Printer,
    command_path: &[String],
    current_executable: &Path,
    systems_directory: &Path,
    current_directory: &Path,
) -> anyhow::Result<Vec<PathBuf>> {
    let exe_dir = current_executable
        .parent()
        .context("The current exectuable has no parent")?;

    fn compute_path(base: &Path, input: &Path, prefix: &str) -> Option<PathBuf> {
        let input = PathBuf::from(input);
        input.strip_prefix(prefix).ok().map(|p| base.join(p))
    }

    command_path
        .iter()
        .map(|path| {
            let path = PathBuf::from(path);
            printer.trace(&format!("generating command path for {path:?}"));
            if let Some(p) = compute_path(exe_dir, &path, "{EXE_DIR}") {
                Ok(p)
            } else if let Some(p) = compute_path(systems_directory, &path, "{SYSTEMS_DIR}") {
                Ok(p)
            } else if let Some(p) = compute_path(current_directory, &path, ".") {
                Ok(p)
            } else if path.has_root() {
                Ok(path.to_owned())
            } else {
                Err(anyhow::anyhow!("Failed to interpret command path {path:?}"))
            }
        })
        .collect()
}

#[test]
fn test_generate_full_command_path_exe() {
    let printer = crate::printer::Printer::new(&crate::printer::LogLevel::Trace);
    assert_eq!(
        generate_full_command_path(
            &printer,
            &["./foo".to_string(), "{EXE_DIR}/bar".to_string(), "{SYSTEMS_DIR}/baz".to_string()],
            &PathBuf::from("/current_exe/cleanroom.exe"),
            &PathBuf::from("/systems"),
            &PathBuf::from("/home/someone")
        ).unwrap(),
        [
            PathBuf::from("/home/someone/foo"),
            PathBuf::from("/current_exe/bar"),
            PathBuf::from("/systems/baz")
        ]
    );
}
